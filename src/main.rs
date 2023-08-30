use std::{
  fs::File,
  io::Write,
};

use futures::StreamExt;
use hyper::{
  Request, Response, Method, StatusCode,
  service::{service_fn, make_service_fn}, Body, Server,
};
use log::{ info, trace, error };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  env_logger::init();

  let addr = ([127, 0, 0, 1], 3222).into();

  let server = Server::bind(&addr).serve(make_service_fn(|_conn|
    async { Ok::<_, hyper::http::Error>(service_fn(image)) }
  ));

  info!("Starting server at: {:?}", addr);

  server.await?;

  Ok(())
}

async fn image(req: Request<Body>) ->  Result<Response<Body>, hyper::http::Error>  {
  match (req.method(), req.uri().path()) {
    (&Method::PUT, "/") => {
      info!("Received PUT request with content_length: {:?}", req.headers().get(hyper::header::CONTENT_LENGTH).unwrap());

      let Ok(file) = File::create("./output.png") else {
        error!("Couldn't open file for write");

        return Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("Couldn't open file for write"));
      };

      match req.into_body().scan((file, false), |(file, ref mut was_err), chunk| {
        if *was_err { return futures::future::ready(None) }

        let result = (|| {
          let chunk = chunk?;
          trace!("Writing {} bytes", chunk.len());
          file.write_all(&chunk)?;
          anyhow::Ok(())
        })();

        if let Err(_) = result { *was_err = true; }

        futures::future::ready(Some(result))
      }).collect::<Vec<_>>().await.into_iter().collect::<anyhow::Result<Vec<_>>>() {
        Ok(_)  => Response::builder().status(StatusCode::OK).body(Body::empty()),
        Err(_) => {
          error!("Error writing to file");
          Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("Error writing to file"))
        },
      }
    },
    _ => Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty())
  }
}
