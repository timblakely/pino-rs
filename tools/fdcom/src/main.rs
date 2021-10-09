use std::{
    fmt::Display,
    str, thread,
    time::{Duration, SystemTime},
};

use clap::{App, Arg, SubCommand};
use serialport::SerialPort;

use messages::Message;

enum FdcanUSBMessage {
    Ok,
    Receive(FdcanMessage),
}

struct FdcanMessage {
    id: Message,
    payload: Vec<u8>,
}

impl Display for FdcanMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "can ext {:#01x} ", self.id as u16)?;
        for byte in &self.payload {
            write!(f, "{:02x?}", byte)?;
        }
        write!(f, " BFr\n")
    }
}

impl FdcanMessage {
    pub fn new(id: Message, payload: Vec<u8>) -> FdcanMessage {
        FdcanMessage { id, payload }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        format!("{}", self).into_bytes()
    }
}

fn time_it<T>(description: &str, callback: T)
where
    T: FnOnce(),
{
    let now = SystemTime::now();
    callback();
    if let Ok(elapsed) = now.elapsed() {
        println!("{} took {:?}", description, elapsed);
    }
}

fn e_zero(port: &mut dyn SerialPort) {
    let msg = FdcanMessage::new(
        Message::CalibrateEZero,
        [1f32.to_le_bytes(), 1f32.to_le_bytes(), 0f32.to_le_bytes()].concat(),
    );

    println!("Writing bytes: {}", msg);

    let mut buffer: Vec<u8> = vec![0; 4096];
    let mut num_read: usize = 0;

    time_it("Write", || {
        time_it("write", || {
            port.write(&msg.to_bytes()).expect("Failed to write");
        });
    });

    thread::sleep(Duration::from_millis(10));

    time_it("WriteWriteWrite", || {
        time_it("Write", || {
            port.write(&msg.to_bytes()).expect("Failed to write");
        });
        thread::sleep(Duration::from_millis(10));
        time_it("Write", || {
            port.write(&msg.to_bytes()).expect("Failed to write");
        });
        thread::sleep(Duration::from_millis(10));
        time_it("Write", || {
            port.write(&msg.to_bytes()).expect("Failed to write");
        });
        thread::sleep(Duration::from_millis(10));
    });

    thread::sleep(Duration::from_millis(10));
    time_it("Read", || {
        num_read = port
            .read(buffer.as_mut_slice())
            .expect("Could not read 'OK'");
        println!(
            "Read {} bytes: {:?}",
            num_read,
            str::from_utf8(&buffer[..num_read])
        );
    });
}

fn main() {
    let matches = App::new("Fdcom")
        .version("0.0.1")
        .about("Tool to communicate with BLDC via FDCANUSB")
        .subcommand(SubCommand::with_name("list").about("show available ports"))
        .arg(Arg::with_name("port").short("p").value_name("PORT"))
        .subcommand(SubCommand::with_name("ezero").about("dummy signal for Ezero"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("list") {
        println!("Available ports:");
        let ports = serialport::available_ports().expect("Couldn't poll ports");
        for p in ports {
            println!("  {}", p.port_name);
        }
        return;
    }

    let port_name = matches.value_of("port").unwrap();
    let mut port = serialport::new(port_name, 100_000_000)
        .timeout(Duration::from_millis(1000))
        .flow_control(serialport::FlowControl::None)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .open()
        .expect(format!("Could not open port {}", port_name).as_str());

    if matches.is_present("ezero") {
        return e_zero(&mut *port);
    }
}
