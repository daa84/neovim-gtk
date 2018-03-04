use std::collections::HashMap;

use neovim_lib::{NeovimApi, Value};

use nvim::NeovimRef;

/// A subscription to a Neovim autocmd event.
struct Subscription {
    /// A callback to be executed each time the event triggers.
    cb: Box<Fn(Vec<String>) + 'static>,
    /// A list of expressions which will be evaluated when the event triggers. The result is passed
    /// to the callback.
    args: Vec<String>,
}

/// A map of all registered subscriptions.
pub struct Subscriptions(HashMap<String, Vec<Subscription>>);

/// A handle to identify a `Subscription` within the `Subscriptions` map.
///
/// Can be used to trigger the subscription manually even when the event was not triggered.
///
/// Could be used in the future to suspend individual subscriptions.
#[derive(Debug)]
pub struct SubscriptionHandle {
    event_name: String,
    index: usize,
}

impl Subscriptions {
    pub fn new() -> Self {
        Subscriptions(HashMap::new())
    }

    /// Subscribe to a Neovim autocmd event.
    ///
    /// Subscriptions are not active immediately but only after `set_autocmds` is called. At the
    /// moment, all calls to `subscribe` must be made before calling `set_autocmds`.
    ///
    /// This function is wrapped by `shell::State`.
    ///
    /// # Arguments:
    ///
    /// - `event_name`: The event to register.
    ///   See `:help autocmd-events` for a list of supported event names. Event names can be
    ///   comma-separated.
    ///
    /// - `args`: A list of expressions to be evaluated when the event triggers.
    ///   Expressions are evaluated using Vimscript. The results are passed to the callback as a
    ///   list of Strings.
    ///   This is especially useful as `Neovim::eval` is synchronous and might block if called from
    ///   the callback function; so always use the `args` mechanism instead.
    ///
    /// - `cb`: The callback function.
    ///   This will be called each time the event triggers or when `run_now` is called.
    ///   It is passed a vector with the results of the evaluated expressions given with `args`.
    ///
    /// # Example
    ///
    /// Call a function each time a buffer is entered or the current working directory is changed.
    /// Pass the current buffer name and directory to the callback.
    /// ```
    /// let my_subscription = shell.state.borrow()
    ///     .subscribe("BufEnter,DirChanged", &["expand(@%)", "getcwd()"], move |args| {
    ///         let filename = &args[0];
    ///         let dir = &args[1];
    ///         // do stuff
    ///     });
    /// ```
    pub fn subscribe<F>(&mut self, event_name: &str, args: &[&str], cb: F) -> SubscriptionHandle
    where
        F: Fn(Vec<String>) + 'static,
    {
        let entry = self.0.entry(event_name.to_owned()).or_insert(Vec::new());
        let index = entry.len();
        entry.push(Subscription {
            cb: Box::new(cb),
            args: args.into_iter().map(|&s| s.to_owned()).collect(),
        });
        SubscriptionHandle {
            event_name: event_name.to_owned(),
            index,
        }
    }

    /// Register all subscriptions with Neovim.
    ///
    /// This function is wrapped by `shell::State`.
    pub fn set_autocmds(&self, nvim: &mut NeovimRef) {
        for (event_name, subscriptions) in &self.0 {
            for (i, subscription) in subscriptions.iter().enumerate() {
                let args = subscription
                    .args
                    .iter()
                    .fold("".to_owned(), |acc, arg| acc + ", " + &arg);
                nvim.command(&format!(
                    "au {} * call rpcnotify(1, 'subscription', '{}', {} {})",
                    event_name, event_name, i, args,
                )).expect("Could not set autocmd");
            }
        }
    }

    /// Trigger given event.
    fn on_notify(&self, event_name: &str, index: usize, args: Vec<String>) {
        if let Some(subscription) = self.0.get(event_name).and_then(|v| v.get(index)) {
            (*subscription.cb)(args);
        }
    }

    /// Wrapper around `on_notify` for easy calling with a `neovim_lib::Handler` implementation.
    ///
    /// This function is wrapped by `shell::State`.
    pub fn notify(&self, params: Vec<Value>) -> Result<(), String> {
        let mut params_iter = params.into_iter();
        let ev_name = params_iter.next();
        let ev_name = ev_name
            .as_ref()
            .and_then(Value::as_str)
            .ok_or("Error reading event name")?;
        let index = params_iter
            .next()
            .and_then(|i| i.as_u64())
            .ok_or("Error reading index")? as usize;
        let args = params_iter
            .map(|arg| arg.as_str().map(|s| s.to_owned()))
            .collect::<Option<Vec<String>>>()
            .ok_or("Error reading args")?;
        self.on_notify(ev_name, index, args);
        Ok(())
    }

    /// Manually trigger the given subscription.
    ///
    /// The `nvim` instance is needed to evaluate the `args` expressions.
    ///
    /// This function is wrapped by `shell::State`.
    pub fn run_now(&self, handle: &SubscriptionHandle, nvim: &mut NeovimRef) {
        let subscription = &self.0.get(&handle.event_name).unwrap()[handle.index];
        let args = subscription
            .args
            .iter()
            .map(|arg| nvim.eval(arg))
            .map(|res| {
                res.ok()
                    .and_then(|val| val.as_str().map(|s: &str| s.to_owned()))
            })
            .collect::<Option<Vec<String>>>();
        if let Some(args) = args {
            self.on_notify(&handle.event_name, handle.index, args);
        } else {
            error!("Error manually running {:?}", handle);
        }
    }
}
