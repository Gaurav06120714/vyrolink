// Loads the Rust core (WebAssembly) for the web chat; see vlweb.js.
import init, { WebNode } from './vlcore/vl_wasm.js';

init().then(() => {
  window.VlWebNode = WebNode;
  window.vlcoreLoaded[0]();
}, window.vlcoreLoaded[1]);
