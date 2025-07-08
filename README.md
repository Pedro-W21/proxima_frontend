# Proxima Frontend

This is a potential frontend for the Proxima system ([link](https://github.com/Pedro-W21/proxima_backend)), built in Tauri + Yew, it is currently very experimental and missing support for multiple Proxima features

## Security

Just like for the current Proxima backend, this has mostly non-existent security, everything happens in HTTP by default, and bugs may expose Proxima's information in certain menus where they are not supposed to be, this is not a production app for now

## Running

To build and run this project :
- clone this repository locally
- install Tauri ([link](https://v2.tauri.app/start/prerequisites/))
- start your proxima backend server
- run `cargo tauri dev` in the root folder