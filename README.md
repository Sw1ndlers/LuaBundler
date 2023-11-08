# Lua Bundler

A customizable Lua Bundler made in Rust  

## Setup
1. Run `cargo install luabundle`, it should download to your cargo bin folder.
2. Run `luabundle` in the command prompt to create a new project and in the current directory.  
3. Run `luabundle` again to bundle your code to the output file.  

## Options
- Require Function (default `loadmodule`)
- Entry File (default `main.lua`)
- Output File (default `LuaBundler/bundled.lua`)
- Minify (default `false`)
- Beautify (default `true`)

## Usage
Create a file called `main.lua` (or what you set as the `Entry File`) in the root folder.  
Files that are being required should be treated like a module script.  
Use the `loadmodule` function and pass in a path to a file (paths are relative to the current file).  
Use `@` before the path to access the root, e.g., `loadmodule("@fileAtRootFolder.lua")`.  


## Example
Suppose you had a file layout like this

```
Project Directory/
├── main.lua
└── utils/
    └── fancyprint.lua
```

Within main.lua
```lua
local fancyprint = loadmodule("utils/fancyprint.lua")
fancyprint("Hello world!")
```

Within fancyprint.lua
```lua
local function fancyprint(text)
  print(text + " was printed with fancy text")
```

Upon running luabundle, the output file would contain the runnable lua code 





