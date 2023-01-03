# tauri-glue
proc macro support for interfacing between rust frontends and tauri. this is quite experimental, and I have yet to add complex type support, but it should work with most primitives.

## Example usage
1. create a tauri command:
   ```rust
   #[tauri::command]
   fn hello(name: &str) -> Result<String, String> {
     Ok(format!("Hello from Tauri, {name} :P"))
   }
   ```
2. in the frontend, include the dependency:
   ```tauri-glue = { git = "https://github.com/DPM97/tauri-glue" }```
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