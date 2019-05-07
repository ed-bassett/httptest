#![feature(box_syntax)]
#![feature(impl_trait_in_bindings)]

use log::{ info, trace, warn, error };
use futures::{ future, prelude::* };
use hyper::{
  Body, Request, Response, Server, Method, StatusCode,
  rt::Future,
  service::service_fn,
};
use std::{
  fs::File,
  io::prelude::*,
};

fn main() {
  env_logger::init();

  let addr    = ([127, 0, 0, 1], 3222).into();
  info!("Starting server at: {:?}", addr);
  let server  = Server::bind(&addr)
    .serve( || service_fn(image))
    .map_err( |e| error!("server error: {}", e) );

  hyper::rt::run(server);
}

type BoxFut = Box<Future<Item=Response<Body>, Error=hyper::http::Error> + Send>;

fn image(req: Request<Body>) -> BoxFut {
  match (req.method(), req.uri().path()) {
    (&Method::PUT, "/") => {
      info!("Received PUT request with content_length: {:?}", req.headers().get(hyper::header::CONTENT_LENGTH).unwrap());
      match File::create("./output.png") {
        Err(_)    => {
          error!("Couldn't open file for write");
          box future::result(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("Couldn't open file for write")))
        },
        Ok(mut f) => box req.into_body().map_err(|_|()).for_each( move |chunk| {
          trace!("Writing {} bytes", chunk.len());
          f.write_all(&chunk).map_err(|_|())
        } ).into_future().then( |r| match r {
          Ok(_)  => Response::builder().status(StatusCode::OK).body(Body::empty()),
          Err(_) => {
            error!("Error writing to file");
            Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("Error writing to file"))
          },
        } ),
      }
    },
    _ => {
      box future::result(Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()))
    },
  }
}