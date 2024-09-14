use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::sink::SinkExt;

#[tokio::main]
async fn main(){
	let stream = TcpStream::connect("127.0.0.1:13330").await.expect("failed to connect to server");
	/*
	let mut codec = LengthDelimitedCodec::new();
	codec.set_max_frame_length(40_000_000);
	println!("length {}", codec.max_frame_length());
	let mut writer = Framed::new(stream, codec);
	*/

	let mut writer = LengthDelimitedCodec::builder()
		.little_endian()
		.max_frame_length(40_000_000)
		.new_framed(stream);
	let message = vec![b'1'; 20_000_000];
	loop {
		writer.send(message.clone().into()).await.expect("failed to send message");
	}
}
