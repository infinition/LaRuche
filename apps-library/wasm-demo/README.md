# WASM Lab

A minimal App proving that WebAssembly executes inside the same isolated iframe as a JS App.
It computes an integer sum using `WebAssembly.instantiateStreaming`, without a CDN or runtime
download. The tiny module is generated from documented WASM bytes by the build script.

From the repository root:

```powershell
python apps-library/wasm-demo/build.py
```

Install `apps-library/wasm-demo/dist/laruche-wasm-demo-1.0.0.laruche-app`, enable **WASM Lab**,
then open it. The initial result should be **42** and the status **WebAssembly actif**.
Both **Panneau** and **Fenêtre** support the same module.

For real applications, compile a browser-targeted `.wasm` module using your preferred toolchain
and place it under `ui/` alongside its generated JavaScript glue. Fetch it with
`credentials: 'omit'`. Bundle all dependencies locally. JavaScript `eval`, arbitrary network
access, shared-memory threads and a native WASI backend are not enabled by this example.
