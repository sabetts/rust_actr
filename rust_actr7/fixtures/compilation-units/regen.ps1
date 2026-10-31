# Regenerate the project-authored production-compilation unit traces.
$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here "..\\..\\..")
$actr = if ($env:ACTR_DIR) { $env:ACTR_DIR } else { Join-Path $root "actr7.x" }
$env:TRACE_MANIFEST = Join-Path $here "manifest.lisp"
$env:TRACE_OUTDIR = if ($args.Count -gt 0) { $args[0] } else { Join-Path $here "..\\compilation-traces" }
$sbcl = if ($env:SBCL) { $env:SBCL } else { "sbcl" }
New-Item -ItemType Directory -Force -Path $env:TRACE_OUTDIR | Out-Null
& $sbcl --non-interactive --disable-debugger `
  --load (Join-Path $actr "load-single-threaded-act-r.lisp") `
  --load (Join-Path $here "..\\run-traces.lisp") `
  --eval "(trace-main)"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
