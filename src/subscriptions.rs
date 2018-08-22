use std::collections::HashMap;

use neovim_lib::{NeovimApi, NeovimApiAsync, Value};

use nvim::{ErrorReport, NeovimRef};

/// A subscription to a Neovim autocmd event.
struct Subscription {
    /// A callback to be executed each time the event triggers.
    cb: Box<Fn(Vec<String>) + 'static>,
    /// A list of expressions which will be evaluated when the event triggers. The result is passed
    /// to the callback.
    args: Vec<String>,
}

/// Subscription keys represent a NeoVim event coupled with a matching pattern. It is expected for
/// the pattern more often than not to be `"*"`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SubscriptionKey {
    event_name: String,
    pattern: String,
}

impl<'a> From<&'a str> for SubscriptionKey {

    fn from(event_name: &'a str) -> Self {
        SubscriptionKey {
            event_name: event_name.to_owned(),
            pattern: "*".to_owned(),
        }
    }
}

impl SubscriptionKey {

    pub fn with_pattern(event_name: &str, pattern: &str) -> Self {
        SubscriptionKey {
            event_name: event_name.to_owned(),
            pattern: pattern.to_owned(),
        }
    }
}

/// A map of all registered subscriptions.
pub struct Subscriptions(HashMap<SubscriptionKey, Vec<Subscription>>);

/// A handle to identify a `Subscription` within the `Subscriptions` map.
///
/// Can be used to trigger the subscription manually even when the event was not triggered.
///
/// Could be used in the future to suspend individual subscriptions.
#[derive(Debug)]
pub struct SubscriptionHandle {
    key: SubscriptionKey,
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
    /// - `key`: The subscription key to register.
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
    pub fn subscribe<F>(&mut self, key: SubscriptionKey, args: &[&str], cb: F) -> SubscriptionHandle
    where
        F: Fn(Vec<String>) + 'static,
    {
        let entry = self.0.entry(key.clone()).or_insert(Vec::new());
        let index = entry.len();
        entry.push(Subscription {
            cb: Box::new(cb),
            args: args.into_iter().map(|&s| s.to_owned()).collect(),
        });
        SubscriptionHandle {
            key,
            index,
        }
    }

    /// Register all subscriptions with Neovim.
    ///
    /// This function is wrapped by `shell::State`.
    pub fn set_autocmds(&self, nvim: &mut NeovimRef) {
        for (key, subscriptions) in &self.0 {
            let SubscriptionKey { event_name, pattern } = key;
            for (i, subscription) in subscriptions.iter().enumerate() {
                let args = subscription
                    .args
                    .iter()
                    .fold("".to_owned(), |acc, arg| acc + ", " + &arg);
                let autocmd = format!(
                    "autocmd {} {} call rpcnotify(1, 'subscription', '{}', '{}', {} {})",
                    event_name, pattern, event_name, pattern, i, args,
                );
                nvim.command_async(&autocmd).cb(|r| r.report_err())
                    .call();
            }
        }
    }

    /// Trigger given event.
    fn on_notify(&self, key: &SubscriptionKey, index: usize, args: Vec<String>) {
        if let Some(subscription) = self.0.get(key).and_then(|v| v.get(index)) {
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
        let pattern = params_iter.next();
        let pattern = pattern
            .as_ref()
            .and_then(Value::as_str)
            .ok_or("Error reading pattern")?;
        let key = SubscriptionKey {
            event_name: String::from(ev_name),
            pattern: String::from(pattern)
        };
        let index = params_iter
            .next()
            .and_then(|i| i.as_u64())
            .ok_or("Error reading index")? as usize;
        let args = params_iter
            .map(|arg| {
                arg
                    .as_str()
                    .map(|s: &str| s.to_owned())
                    .or_else(|| arg.as_u64().map(|uint: u64| format!("{}", uint)))
            })
            .collect::<Option<Vec<String>>>()
            .ok_or("Error reading args")?;
        self.on_notify(&key, index, args);
        Ok(())
    }

    /// Manually trigger the given subscription.
    ///
    /// The `nvim` instance is needed to evaluate the `args` expressions.
    ///
    /// This function is wrapped by `shell::State`.
    pub fn run_now(&self, handle: &SubscriptionHandle, nvim: &mut NeovimRef) {
        let subscription = &self.0.get(&handle.key).unwrap()[handle.index];
        let args = subscription
            .args
            .iter()
            .map(|arg| nvim.eval(arg))
            .map(|res| {
                res.ok()
                    .and_then(|val| {
                        val
                            .as_str()
                            .map(|s: &str| s.to_owned())
                            .or_else(|| val.as_u64().map(|uint: u64| format!("{}", uint)))
                    })
            })
            .collect::<Option<Vec<String>>>();
        if let Some(args) = args {
            self.on_notify(&handle.key, handle.index, args);
        } else {
            error!("Error manually running {:?}", handle);
        }
    }
}
