use std::{
  fs::File,
  io::Write, net::SocketAddr,
};

use futures::StreamExt;
use tracing::{info, trace, error};

use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Request, Response, Method, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let subscriber = tracing_subscriber::fmt().finish();
  tracing::subscriber::set_global_default(subscriber)?;

  let addr = SocketAddr::from(([127, 0, 0, 1], 3222));

  let listener = TcpListener::bind(addr).await?;

  info!("Starting server at: {:?}", addr);

  loop {
    let (stream, _) = listener.accept().await?;

    // Use an adapter to access something implementing `tokio::io` traits as if they implement
    // `hyper::rt` IO traits.
    let io = TokioIo::new(stream);

    // Spawn a tokio task to serve multiple connections concurrently
    tokio::task::spawn(async move {
      if let Err(err) = http1::Builder::new()
        .serve_connection(io, service_fn(image))
        .await
      {
        eprintln!("Error serving connection: {:?}", err);
      }
    });
  }
}

async fn image(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, hyper::http::Error> {
  match (req.method(), req.uri().path()) {
    (&Method::PUT, "/") => {
      info!("Received PUT request with content_length: {:?}", req.headers().get(hyper::header::CONTENT_LENGTH).unwrap());

      let Ok(file) = File::create("./output.png") else {
        error!("Couldn't open file for write");

        return Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Full::new(Bytes::from("Couldn't open file for write")));
      };

      match req.into_body().into_data_stream().scan((file, false), |(file, ref mut was_err), chunk| {
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
        Ok(_)  => Response::builder().status(StatusCode::OK).body(Full::new(Bytes::new())),
        Err(_) => {
          error!("Error writing to file");
          Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Full::new(Bytes::from("Error writing to file")))
        },
      }
    },
    _ => Response::builder().status(StatusCode::NOT_FOUND).body(Full::new(Bytes::new()))
  }
}
