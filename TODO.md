# Possible Additional Functionality and Improvements

## IU testing on Linux

We should check how the app behaves on Linux.

## Documentation changes

Add warnings that on macOS the default terminal (unlike ghostty) does not map some characters, such as Ctrl+Home and Shift-Down keys, so mapping to terminal escape sequences need to be added explicitly.

## Add a better story for lexing to color the syntax

Our lexical analysis is proof of concept only; many things are not supported.

In particular, some lightweight support for markdown would be useful.
A simple way is to add generic notion of a section headers and make it work for markdown only for now.

## Load color palette

Note everyone may like the current color palette and black and white terminals might now work well with it.

Enable loading colors from a file.

## Load keywords

Enable loading keywords from a file for a given file extension.
