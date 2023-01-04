# tauri-glue
 a proc macro for interfacing between rust frontends (e.g., [leptos](https://github.com/gbj/leptos)) and [tauri](https://github.com/tauri-apps/tauri) (i.e., a rust-based desktop-app backend). at the moment it's limited to types that implement    ```wasm_bindgen::convert::IntoWasmAbi``` :)

## Example usage
1. create a tauri command:
   ```rust
   use tauri_glue::*;
   
   ...
   
   #[tauri::command]
   fn hello(name: &str) -> Result<String, String> {
     Ok(format!("Hello from Tauri, {name} :P"))
   }
   ```
2. in the frontend, include the dependency:
   ```toml
   tauri-glue = { git = "https://github.com/DPM97/tauri-glue" }
   ```
3. before calling the command, create the bindings:
   ```rust
   #[tauri_glue::bind_command(name = hello)]
   pub async fn hello(name: String) -> Result<JsValue, JsValue>;
   ```
4. interface with the command like normal
   ```rust
   match hello("example_name_sent_from_frontend".to_string()).await {
     Ok(resp) => {
         ...
     }
     Err(e) => {
         ...
     }
   }
   ```
5. ![example](./example.png)