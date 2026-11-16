use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct Config {
    pub port: Option<String>,
    pub smoke_test: bool,
    pub invert_roll: bool,
    pub invert_pitch: bool,
    pub roll_offset: f32,
    pub pitch_offset: f32,
    pub response: f32,
}
impl Config {
    pub fn from_args() -> Result<Self, String> {
        let mut config = Self {
            port: None,
            smoke_test: false,
            invert_roll: false,
            invert_pitch: false,
            roll_offset: 0.,
            pitch_offset: 0.,
            response: 12.,
        };
        let mut demo = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--demo" => demo = true,
                "--smoke-test" => config.smoke_test = true,
                "--invert-roll" => config.invert_roll = true,
                "--invert-pitch" => config.invert_pitch = true,
                "--roll-offset" | "--pitch-offset" | "--response" => {
                    let n = args
                        .next()
                        .and_then(|v| v.parse::<f32>().ok())
                        .filter(|v| v.is_finite() && v.abs() <= 360.)
                        .ok_or_else(|| format!("Invalid value for {arg}"))?;
                    match arg.as_str() {
                        "--roll-offset" => config.roll_offset = n,
                        "--pitch-offset" => config.pitch_offset = n,
                        _ if n > 0. => config.response = n,
                        _ => return Err("Response must be positive".into()),
                    }
                }
                "--help" | "-h" => return Err(String::new()),
                _ if !arg.starts_with('-') && config.port.is_none() => config.port = Some(arg),
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        if config.smoke_test && !demo {
            return Err("--smoke-test requires --demo".into());
        }
        if demo && config.port.is_some() {
            return Err("Use a serial port OR --demo".into());
        }
        if !demo && config.port.is_none() {
            return Err("Specify a serial port or --demo".into());
        }
        Ok(config)
    }
    pub fn demo(&self) -> bool {
        self.port.is_none()
    }
}
