use std::{env, path::PathBuf};

pub struct Config {
    pub directory: Box<PathBuf>,
}

impl Config {
    pub fn new(mut args: impl Iterator<Item = String>) -> Self {
        let mut directory = PathBuf::from(format!(
            "{}/public",
            env::current_dir().unwrap().to_str().unwrap()
        ));

        while let Some(arg) = args.next() {
            if arg.as_str() == "--directory" {
                directory = PathBuf::from(args.next().expect("invalid args"));
            }
        }

        Self {
            directory: Box::new(directory),
        }
    }
}
