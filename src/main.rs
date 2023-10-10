#![allow(unused_parens)]

// Library Imports

mod utils;
use crate::utils::{
    format_file, 
    get_macros, 
    require_function_commented, 
    split,
    Macro
};

use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use std::{env, fs, path::PathBuf, fmt::Debug};
use serde::{Deserialize, Serialize};

use color_print::cprintln;

use clap::Parser;
use clearscreen;
use serde_json;
use anyhow;

// make windows support ansi colors | REG ADD HKCU\CONSOLE /f /v VirtualTerminalLevel /t REG_DWORD /d 1

const WORKSPACE_FOLDER: &str = "\\Packages\\ROBLOXCORPORATION.ROBLOX_55nm5eh3cm0pr\\AC\\workspace\\";

// Structs
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ConfigStruct {
    require_function: String,
    entry_file: String,
    output_file: String,

    minify: bool,
    beautify: bool,
}


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    active: bool,   
}

// Functions

fn get_relative_folder(input_file: &PathBuf) -> PathBuf {
    let mut relative_folder = input_file.to_path_buf(); // root\folder\module.lua
    relative_folder = relative_folder.parent().unwrap().to_path_buf(); // root\folder
    relative_folder = PathBuf::from(relative_folder);
    return relative_folder;
}

fn get_require_content(line: &String, require_function: &String) -> String {
    let require_split = format!("{}(", require_function); // loadmodule(
    let require_content = &split(&line, &require_split)[1]; // "module.lua")
    let require_content = &split(require_content, ")")[0].clone(); // "module.lua"
    
    return require_content.to_string();
}

fn get_require_arguments(require_content: &String) -> String {
    let mut arguments = split(&require_content, ","); // "module.lua", arg1, arg2
    arguments.remove(0); // removing "module.lua"

    return arguments.join(","); // arg2, arg3
}

fn parse(root_path: &PathBuf, input_file: PathBuf, require_function: &String) -> String {
    let mut root_path = root_path.clone();
    let file_name = input_file.file_name().unwrap().to_str().unwrap();

    let input_string = fs::read_to_string(&input_file).unwrap();
    let input_string = input_string.replace("{{filename}}", file_name); // filename global variable

    let mut lines: Vec<String> = split(&input_string, "\n");

    let (macros, new_lines) = get_macros(&lines); // remove comments and get macros
    lines = new_lines;

    lines = lines.iter().map(|s| s.trim().to_string()).collect(); // remove whitespace
    lines.retain(|x| !x.is_empty()); // remove empty lines

    let mut new_lines: Vec<String> = Vec::new();

    for (i, mut line) in lines.iter().enumerate() {
        if line.contains(require_function) {
            if require_function_commented(line.clone(), require_function.clone()) {
                new_lines.push(line.to_string());
                continue;
            }

            let mut add_semicolon = false;

            if i != 0 {
                if let Some(line_before) = lines.get(i - 1) {
                    let line_before = line_before.trim();
                    add_semicolon = line_before.ends_with(")") && !line.contains("=");
                }
            }

            let mut macro_types: Vec<Macro> = match macros.get(&i) {
                Some(macro_types) => macro_types.to_owned(),
                None => Vec::new(),
            };

            let relative_folder = get_relative_folder(&input_file);
            let require_content = get_require_content(&line, &require_function);
            let arguments = get_require_arguments(&require_content);

            let mut line_replace = line.replace(&arguments, "");
            line_replace = line_replace.replace(",", ""); 
            line = &line_replace; 

            let mut require_content = require_content.trim_end_matches(&arguments); // removing arguments from end
            require_content = require_content.trim_matches(|c| c == '"' || c == '\'' || c == ','); // removing quotes and comma's from start and end
            require_content = require_content.trim_matches(|c| c == '"' || c == '\''); // removing extra double quotes (im lazy)

            let has_at_symbol = require_content.contains("@");
            if has_at_symbol { 
                require_content = require_content.trim_start_matches("@");
                macro_types.push(Macro::AbsPath);
            }

            if macro_types.contains(&Macro::AbsPath) == false {
                root_path = relative_folder.clone();
            }

            let require_path = root_path.join(require_content);
            if !require_path.is_file() {
                println!("File not found: {}", require_path.display());
                std::process::exit(1);
            }

            let whole_function = format!(
                "{function}(\"{at_symbol}{content}\")", 
                function = require_function,
                at_symbol = if has_at_symbol { "@" } else { "" },
                content = require_content
            
            ); // loadmodule("module.lua")

            // let path_comment = format!("_[[{}]];\n", relative_file_name.display()); // cant add regular comment because darklua removes them
            let function_call_args = format!("({})", arguments);

            let output = format!(
                "{semicolon}(function(...) {content} end){function_call}",

                semicolon = (if add_semicolon { ";" } else { "" }),
                content = (parse(&root_path, require_path, require_function)),
                function_call = (&function_call_args)
            );

            let output = line.replace(&whole_function, output.as_str());
            new_lines.push(output);
        } else {
            new_lines.push(line.to_string());
        }
    }

    return new_lines.join("\n");
}

fn bundle(config: &ConfigStruct) {
    let root_path = env::current_dir().unwrap();
    let entry_file = root_path.join(&config.entry_file);

    if !entry_file.is_file() {
        println!("Entry File {:?} not found, make sure it exists in the root directory", entry_file.file_name().unwrap());
        std::process::exit(1);
    }

    let output = parse(&root_path, entry_file, &config.require_function);
    let output = format!("-- Bundled with LuaBundle\n\n{}", output);

    fs::write(root_path.join(&config.output_file), output).unwrap();

    // ---------- minify or beautify ----------
    
    if config.minify || config.beautify {
        println!("Formatting...");
        format_file(&PathBuf::from(&config.output_file), config.minify, config.beautify)
    }
}

// ignore this, made very shittily
fn handle_active_bundling() {
    std::io::stdin().read_line(&mut String::new()).unwrap();    // wait for input from console

    let start = std::time::Instant::now();

    clearscreen::clear().unwrap();
    println!("Bundling...");

    let root_path = env::current_dir().unwrap();

    let config_path = root_path.join("LuaBundler/config.json");
    let config_string = fs::read_to_string(config_path).unwrap();
    let config: ConfigStruct = serde_json::from_str(&config_string).unwrap();

    bundle(&config.clone());

    // Read output file and send it to the client
    let output_file_path = root_path.join(&config.output_file);
    let output_file = fs::read_to_string(output_file_path).unwrap();

    // send code to workspace folder
    let roblox_path = env::var("LOCALAPPDATA").unwrap() + WORKSPACE_FOLDER;
    let roblox_path = PathBuf::from(roblox_path);

    let workspace_output_path = roblox_path.join("bundled.lua");

    fs::write(workspace_output_path, output_file).unwrap();
    
    cprintln!("<green>Bundled in: </green><cyan>{:?}</cyan>", start.elapsed());
}

fn main() -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();

    let args: Args = Args::parse();
    let active_bundling = args.active;

    let root_path = env::current_dir().unwrap();

    let luabundler_path = root_path.join("LuaBundler");
    if !luabundler_path.is_dir() {
        fs::create_dir(luabundler_path).unwrap();
    }

    let config_path = root_path.join("LuaBundler/config.json");
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
            .default("LuaBundler/bundled.lua".to_string())
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
        fs::write(root_path.join("LuaBundler/config.json"), json).unwrap();

        cprintln!("\n<bold><green>Setup complete!</green> Run the program again to bundle your code.</>\nPress Enter to Exit");
        std::io::stdin().read_line(&mut String::new()).unwrap();
        std::process::exit(0);
    } else {
        let config_string = fs::read_to_string(config_path).unwrap();
        config = serde_json::from_str(&config_string).unwrap();
    }

    if active_bundling {
        cprintln!("<bold> Active Bundling Enabled </>\n");
        loop {
            handle_active_bundling();
        }
    }

    bundle(&config.clone());

    cprintln!("<green>Bundled in: </green><cyan>{:?}</cyan>", start.elapsed());
    
    Ok(())
}