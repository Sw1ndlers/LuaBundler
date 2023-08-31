#![allow(unused_parens)]

// Library Imports
use std::{env, fs, path::PathBuf, fmt::Debug, collections::HashMap};
use anyhow;
use clap::Parser;

use serde::{Deserialize, Serialize};
use serde_json;

use stacker;
use clearscreen;

use color_print::cprintln;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use darklua_core::{Configuration, GeneratorParameters, Resources};

// make windows support ansi colors | REG ADD HKCU\CONSOLE /f /v VirtualTerminalLevel /t REG_DWORD /d 1

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

#[derive(Eq, Hash, PartialEq)]
enum Macro {
    DontRun,
    AbsPath,
    None
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

fn format_file(file: &PathBuf, minify: bool, beautify: bool) {
    let resources = Resources::from_file_system();
    let mut configuration = Configuration::empty();

    if minify {
        configuration = Configuration::empty().with_generator(GeneratorParameters::default_dense());
    }
    if beautify {
        configuration = Configuration::empty().with_generator(GeneratorParameters::default_readable());
    };

    let process_options = darklua_core::Options::new(PathBuf::from(&file))
        .with_output(PathBuf::from(&file))
        .with_configuration(configuration);

    stacker::maybe_grow(1024 * 1024 * 2, 1024 * 1024 * 3, || {
        darklua_core::process(&resources, process_options);
    });
}

fn get_macros(lines: &Vec<String>) -> (HashMap<usize, Macro>, Vec<String>) {
    let mut macro_lines: HashMap<usize, Macro> = HashMap::new();
    let mut new_lines: Vec<String> = Vec::new();

    for (i, line) in lines.clone().iter().enumerate() {
        let mut macro_found = false;

        if line.contains("[dont_run]") {
            macro_lines.insert(i, Macro::DontRun);
            macro_found = true;
        }
        if line.contains("[abs_path]") {
            macro_lines.insert(i, Macro::AbsPath);
            macro_found = true;
        }

        if macro_found {
            let without_comment = &split(line, "--")[0];
            new_lines.push(without_comment.trim().to_string());
        } else {
            new_lines.push(line.to_string());
        }
    }
    return (macro_lines, new_lines);
}

fn parse(root_path: &PathBuf, input_file: PathBuf, require_function: &String) -> String {
    let mut root_path = root_path.clone();

    let input_string = fs::read_to_string(&input_file).unwrap();
    let mut lines: Vec<String> = split(&input_string, "\n");
    
    let (macros, new_lines) = get_macros(&lines); // remove comments and get macros

    lines = new_lines;

    lines = lines.iter().map(|s| s.trim().to_string()).collect(); // remove whitespace
    lines.retain(|x| !x.is_empty()); // remove empty lines

    let mut new_lines: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(require_function) {
            
            if require_function_commented(line.clone(), require_function.clone()) {
                new_lines.push(line.to_string());
                continue;
            }

            let last_line = lines.get(i - 1);
            let mut add_semicolon = false;

            match last_line {
                Some(last_line) => {
                    let last_line = last_line.trim();
                    add_semicolon = (last_line.ends_with(")") && line.contains("=") == false)
                }
                None => {}
            }

            let macro_type = match macros.get(&i) {
                Some(macro_type) => macro_type,
                None => &Macro::None,
            };


            let relative_folder = input_file.to_path_buf(); // root\folder\module.lua
            let relative_folder = relative_folder.parent().unwrap(); // root\folder
            let relative_folder = PathBuf::from(relative_folder); 

            if macro_type != &Macro::AbsPath {
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
                "{semicolon}(function() {content} end){function_call}",

                semicolon = (if add_semicolon { ";" } else { "" }),
                content = (parse(&root_path, require_path, require_function)),
                function_call = (if macro_type != &Macro::DontRun { "()" } else { "" }),
            );

            let output = line.replace(&whole_function, output.as_str());
            new_lines.push(output);
        } else {
            new_lines.push(line.to_string());
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
    
    if config.minify || config.beautify {
        format_file(&PathBuf::from(&config.output_file), config.minify, config.beautify)
    }
}

fn handle_active_bundling() {
    std::io::stdin().read_line(&mut String::new()).unwrap();    // wait for input from console

    let start = std::time::Instant::now();

    clearscreen::clear().unwrap();
    println!("Bundling...");

    let root_path = env::current_dir().unwrap();

    let config_path = root_path.join("lbundle.json");
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
    let config_path = root_path.join("lbundle.json");
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

<bold> Require Function: </> <cyan> {} </cyan>
<bold> Entry File: </> <cyan> {} </cyan>
<bold> Output File: </> <cyan> {} </cyan>
<bold> Minify: </> <cyan> {} </cyan>
<bold> Beautify: </> <cyan> {} </cyan>
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
        std::io::stdin().read_line(&mut String::new()).unwrap();
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

    cprintln!("<green>Bundled in: </green><cyan>{:?}</cyan>", start.elapsed());
    Ok(())
}