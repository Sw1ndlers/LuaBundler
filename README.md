# Lua Bundler

A customizable Lua Bundler made in rust

## Options
Require Function (default `loadmodule`)
Entry File (default `main.lua`)
Output File (default `LuaBundler/bundled.lua`)
Minify (default `false`)
Beautify (deafult `true`)

## Usage
Files that are being required should be treated like a module script
Use the require function and pass in a path to a file `(paths are relative to the current file)`
Use @ before the path to access the root, ex `loadmodule("@fileAtRootFolder.lua")`
