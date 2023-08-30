// Library Imports
use std::{env, fs, path::PathBuf, fmt::Debug};
use anyhow;
use clap::Parser;

use serde::{Deserialize, Serialize};
use serde_json;

use stacker;
use clearscreen;

use color_print::cprintln;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use darklua_core::{Configuration, GeneratorParameters, Resources};


// Structs
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ConfigStruct {
    require_function: String,
    entry_file: String,
    output_file: String,

    minify: bool,
    beautify: bool,
}

#[derive(Debug, clap::Args)]
pub struct Options {
    /// Path to the lua file to minify.
    input_path: PathBuf,
    /// Where to output the result.
    output_path: PathBuf,
    /// The maximum number of characters that should be written on a line.
    #[arg(long)]
    column_span: Option<usize>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    active: bool,   
}

// Functions

fn split(input: &str, to_split: &str) -> Vec<String> {
    let res: Vec<String> = input.split(to_split).map(|s| s.to_string()).collect();
    return res;
}

fn require_function_commented(line: String, require_function: String) -> bool {
    let comment_position = line.find("--");
    let loadmodule_position = line.find(&require_function);

    if comment_position.is_some() && loadmodule_position.is_some() {
        let comment_position = comment_position.unwrap();
        let loadmodule_position = loadmodule_position.unwrap();

        if comment_position < loadmodule_position {
            return true;
        }
    }

    return false;
}

fn parse(root_path: &PathBuf, input_file: PathBuf, require_function: &String) -> String {
    let input_string = fs::read_to_string(&input_file).unwrap();
    let mut root_path = root_path.clone();

    let mut lines: Vec<String> = split(&input_string, "\n");
    lines = lines.iter().map(|s| s.trim().to_string()).collect(); // remove whitespace

    let mut new_lines: Vec<String> = Vec::new();
    for line in lines {
        if line.contains(require_function) {
            
            if require_function_commented(line.clone(), require_function.clone()) {
                new_lines.push(line);
                continue;
            }

            let dont_run = line.contains("[dont_run]");
            let absolute_path = line.contains("[abs_path]");

            let relative_folder = input_file.to_path_buf(); // root\folder\module.lua
            let relative_folder = relative_folder.parent().unwrap(); // root\folder
            let relative_folder = PathBuf::from(relative_folder); 

            if absolute_path == false {
                root_path = relative_folder.clone();
            }

            let require_split = format!("{}(", require_function); // loadmodule(
            let require_content = &split(&line, &require_split)[1]; // "module.lua")
            let require_content = &split(require_content, ")")[0].clone(); // "module.lua"
            let require_content = &require_content.replace("\"", ""); // removing double quotes
            let require_content = &require_content.replace("'", ""); // removing single quotes

            let require_path = root_path.join(require_content);

            if !require_path.is_file() {
                println!("File not found: {}", require_path.display());
                std::process::exit(1);
            }

            let whole_function = format!("loadmodule(\"{}\")", require_content); // loadmodule("module.lua")

            let output = format!(
                "(function() {} end){}",

                parse(&root_path, require_path, require_function),

                if dont_run == false { "()" } else { "" },
            );
            
            let output = line.replace(&whole_function, output.as_str());
            new_lines.push(output);
        } else {
            new_lines.push(line);
        }
    }

    let output = new_lines.join("\n");
    return output;
}

fn bundle(config: &ConfigStruct) {
    let root_path = env::current_dir().unwrap();

    let entry_file = root_path.join(&config.entry_file);

    let output = parse(&root_path, entry_file, &config.require_function);
    fs::write(root_path.join(&config.output_file), output).unwrap();

    // ---------- minify or beautify ----------
    
    let resources = Resources::from_file_system();
    let darklua_options = Options {
        input_path: PathBuf::from(&config.output_file),
        output_path: PathBuf::from(&config.output_file),
        column_span: Some(10),
    };
    
    if config.minify || config.beautify {
        let mut configuration = Configuration::empty();

        if config.minify {
            configuration = Configuration::empty().with_generator(GeneratorParameters::default_dense());
        }
        if config.beautify {
            configuration = Configuration::empty().with_generator(GeneratorParameters::default_readable());
        };
    
        let process_options = darklua_core::Options::new(PathBuf::from(&darklua_options.input_path))
            .with_output(PathBuf::from(&darklua_options.output_path))
            .with_configuration(configuration);

        // darklua_core::process(&resources, process_options);

        stacker::maybe_grow(1024 * 1024 * 2, 1024 * 1024 * 3, || {
            darklua_core::process(&resources, process_options);
        });
    }
    
}

fn handle_active_bundling() {
    std::io::stdin().read_line(&mut String::new()).unwrap();    // wait for input from console

    let start = std::time::Instant::now();

    clearscreen::clear().unwrap();
    println!("Bundling...");

    let root_path = env::current_dir().unwrap();

    let config_path = root_path.join("lbundle_config.json");
    let config_string = fs::read_to_string(config_path).unwrap();
    let config: ConfigStruct = serde_json::from_str(&config_string).unwrap();

    bundle(&config.clone());

    // Read output file and send it to the client
    let output_file_path = root_path.join(&config.output_file);
    let output_file = fs::read_to_string(output_file_path).unwrap();

    // send code to workspace folder
    let roblox_path = env::var("LOCALAPPDATA").unwrap() + "\\Packages\\ROBLOXCORPORATION.ROBLOX_55nm5eh3cm0pr\\AC\\workspace\\";
    let roblox_path = PathBuf::from(roblox_path);

    let roblox_output_file_path = roblox_path.join(&config.output_file);
    fs::write(roblox_output_file_path, output_file).unwrap();
    
    cprintln!("<green>Bundled in: </green><cyan>{:?}</cyan>", start.elapsed());
}

fn main() -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();

    let args: Args = Args::parse();
    let active_bundling = args.active;

    let root_path = env::current_dir().unwrap();
    let config_path = root_path.join("lbundle_config.json");
    let config: ConfigStruct;

    // if config does not exist, create it
    if !config_path.is_file() {
        let require_function = Input::new()
            .with_prompt("Require Function")
            .default("loadmodule".to_string())
            .interact().unwrap();

        let entry_file = Input::new()
            .with_prompt("Entry File")
            .default("main.lua".to_string())
            .interact().unwrap();

        let output_file = Input::new()
            .with_prompt("Output File")
            .default("bundled.lua".to_string())
            .interact().unwrap();

        let minify = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Minify?")
            .default(false)
            .interact()
            .unwrap();

        let beautify = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Beautify?")
            .default(true)
            .interact()
            .unwrap();


        cprintln!(
            "
<bold><green> Do these settings look right? </green> </>

<bold>Require Function: </> <cyan> {} </cyan>
<bold>Entry File: </> <cyan> {} </cyan>
<bold>Output File: </> <cyan> {} </cyan>
<bold>Minify: </> <cyan> {} </cyan>
<bold>Beautify: </> <cyan> {} </cyan>
            ",
            require_function,
            entry_file,
            output_file,
            minify,
            beautify,
        );



        config = ConfigStruct {
            require_function,
            entry_file,
            output_file,
            minify,
            beautify,
        };


        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Confirm?")
            .default(true)
            .interact()
            .unwrap();

        if confirm == false {
            cprintln!("<bright-red>Setup canceled!</bright-red> Press Enter to Exit");
            std::process::exit(0);
        }

        let json = serde_json::to_string(&config).unwrap();
        fs::write(root_path.join("lbundle.json"), json).unwrap();

        cprintln!("\n<bold><green>Setup complete!</green> Run the program again to bundle your code.</>\nPress Enter to Exit");
        std::process::exit(0);
    } else {
        let config_string = fs::read_to_string(config_path).unwrap();
        config = serde_json::from_str(&config_string).unwrap();
    }

    if active_bundling {
        loop {
            handle_active_bundling();
        }
    }

    bundle(&config.clone());

    cprintln!("<green>Bundled in: </green><cyan>{:?}</cyan>\nPress Enter to Exit", start.elapsed());
    std::io::stdin().read_line(&mut String::new()).unwrap();
    Ok(())
}