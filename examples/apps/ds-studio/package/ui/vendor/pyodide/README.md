# Vendored Python runtime

This directory is empty on purpose. DS Studio ships with its built-in kernel
and starts without any download.

To run real Python instead, fill this directory from the repository root:

```powershell
python examples/apps/ds-studio/tools/vendor_pyodide.py
python examples/apps/ds-studio/build.py
```

The App detects `studio-manifest.json` here at startup and switches to the
Python kernel. Remove the files, or run the script with `--clean`, to go back.

The runtime cannot be fetched at runtime: the App sandbox allows script and
network access only to this package's own asset directory, so a CDN load is
blocked. That is also why the 32 MiB package limit is the binding constraint
on which Python packages can be included.
