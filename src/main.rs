#[allow(unused_imports)]
use std::io::{self, Write};

fn main() -> std::io::Result<()> {
    loop {
        // Print the prompt
        print!("$ ");
        io::stdout().flush()?;

        // Wait for user input
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input)?;

        // Enter the main loop
        match input.trim_end_matches('\n') {
            "" => {}
            cmd => {
                println!("{cmd}: command not found");
            }
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}
