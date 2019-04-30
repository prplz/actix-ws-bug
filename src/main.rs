use std::collections::HashSet;
use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

fn ws_index(
    r: HttpRequest,
    stream: web::Payload,
    server: web::Data<Addr<Server>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        Websocket {
            hb: Instant::now(),
            server: server.get_ref().clone(),
        },
        &r,
        stream,
    )
}

struct Websocket {
    hb: Instant,
    server: Addr<Server>,
}

impl Actor for Websocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket timed out!");
                ctx.stop();
            } else {
                ctx.ping("");
            }
        });
        self.server.do_send(Connect(ctx.address()));
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.server.do_send(Disconnect(ctx.address()));
        Running::Stop
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for Websocket {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Pong(_) => self.hb = Instant::now(),
            ws::Message::Close(reason) => {
                println!("Websocket closed! {:?}", reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

#[derive(Message)]
struct Text(String);

impl Handler<Text> for Websocket {
    type Result = ();

    fn handle(&mut self, msg: Text, ctx: &mut Self::Context) {
        ctx.text(msg.0); // comment out this line and the problem goes away
    }
}

struct Server {
    sockets: HashSet<Addr<Websocket>>,
}

impl Actor for Server {
    type Context = Context<Self>;
}

#[derive(Message)]
struct Connect(Addr<Websocket>);

#[derive(Message)]
struct Disconnect(Addr<Websocket>);

impl Handler<Connect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) {
        for socket in &self.sockets {
            socket.do_send(Text("Someone connected".to_owned()));
        }
        self.sockets.insert(msg.0);
        println!("There are {} sockets", self.sockets.len());
    }
}

impl Handler<Disconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) {
        self.sockets.remove(&msg.0);
        println!("There are {} sockets", self.sockets.len());
        for socket in &self.sockets {
            socket.do_send(Text("Someone disconnected".to_owned()));
        }
    }
}

fn main() -> std::io::Result<()> {
    let sys = System::new("ws-example");

    let server = Server {
        sockets: HashSet::new(),
    }
    .start();

    HttpServer::new(move || {
        App::new()
            .data(server.clone())
            .route("/ws/", web::get().to(ws_index))
    })
    .bind("127.0.0.1:8080")?
    .start();

    sys.run()
}
