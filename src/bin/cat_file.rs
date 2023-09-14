use rand::Rng;
use std::{error::Error, fs::OpenOptions, io::Write, thread, time};

fn main() -> Result<(), Box<dyn Error>> {
    let append = "asdf asdflk";
    let file_path = "test-file";

    let mut rng = rand::thread_rng();

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    loop {
        let newline: bool = rng.gen();

        file.write_all(append.as_bytes())?;

        if newline {
            file.write(b"\n")?;
        }

        file.flush()?;

        let delay = time::Duration::from_secs_f32(2.0);
        thread::sleep(delay);
    }
}
