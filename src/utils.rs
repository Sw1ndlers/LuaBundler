use darklua_core::{Configuration, GeneratorParameters, Resources};
use std::{path::PathBuf, collections::HashMap};
use stacker;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum Macro {
    DontRun,
    AbsPath
}

pub fn split(input: &str, to_split: &str) -> Vec<String> {
    // splits a string into a vector of strings
    let res: Vec<String> = input.split(to_split).map(|s| s.to_string()).collect();
    return res;
}

pub fn require_function_commented(line: String, require_function: String) -> bool {
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

pub fn format_file(file: &PathBuf, minify: bool, beautify: bool) {
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

pub fn get_macros(lines: &Vec<String>) -> (HashMap<usize, Vec<Macro>>, Vec<String>) {
    let mut macro_lines: HashMap<usize, Vec<Macro>> = HashMap::new();
    let mut new_lines: Vec<String> = Vec::new();

    for (i, line) in lines.clone().iter().enumerate() {
        let mut macro_found = false;
        let mut macros = Vec::new();

        if line.contains("[abs_path]") {
            macros.push(Macro::AbsPath);
            macro_found = true;
        }

        if macro_found {
            macro_lines.insert(i, macros);

            let without_comment = &split(line, "--")[0];
            new_lines.push(without_comment.trim().to_string());
        } else {
            new_lines.push(line.to_string());
        }
    }
    return (macro_lines, new_lines);
}
