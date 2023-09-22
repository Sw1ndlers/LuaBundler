# Lua Bundler

A customizable Lua Bundler made in rust

## Options
Require Function `(default loadmodule)` <br>
Entry File `(default main.lua)` <br>
Output File `(default LuaBundler/bundled.lua)` <br>
Minify `(default false)` <br>
Beautify `(default true)` <br>

## Usage
Files that are being required should be treated like a module script <br>
Use the require function and pass in a path to a file `(paths are relative to the current file)` <br>
Use @ before the path to access the root, ex `loadmodule("@fileAtRootFolder.lua")` <br>
