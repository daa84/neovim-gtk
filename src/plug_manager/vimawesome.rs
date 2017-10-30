use std::io;
use futures::{Future, Stream};
use hyper::{self, Client};
use tokio_core::reactor::Core;
use serde_json;
use hyper_tls::HttpsConnector;

pub struct Vimawesome {}

impl Vimawesome {
    pub fn new() -> Self {
        Vimawesome {}
    }

    pub fn log(&self) {
        match self.request() {
            Ok(list) => println!("list: {:?}", list),
            Err(e) => error!("{}", e),
        }
    }

    fn request(&self) -> Result<DescriptionList, hyper::error::Error> {
        let mut core = Core::new()?;
        let handle = core.handle();
        let client = Client::configure()
            .connector(HttpsConnector::new(4, &handle).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            })?)
            .build(&handle);
        let uri = "https://vimawesome.com/api/plugins?query=&page=1".parse()?;

        let work = client.get(uri).and_then(|res| {
            res.body().concat2().and_then(move |body| {
                let description_list: DescriptionList =
                    serde_json::from_slice(&body).map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, e)
                    })?;
                Ok(description_list)
            })
        });
        core.run(work)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct DescriptionList {
    plugins: Box<[Description]>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Description {
    name: String,
    github_url: String,
    author: String,
    github_stars: i64,
}
