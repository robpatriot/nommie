use actix_web::{get, App, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Nommie Backend! 🃏")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Starting Nommie Backend on http://127.0.0.1:3001");

    HttpServer::new(|| App::new().service(hello))
        .bind(("127.0.0.1", 3001))?
        .run()
        .await
}
