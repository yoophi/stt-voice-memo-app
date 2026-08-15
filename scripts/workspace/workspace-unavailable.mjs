const [area = "unknown", operation = "unknown"] = process.argv.slice(2);

process.stderr.write(
  `WORKSPACE_RUNTIME_UNAVAILABLE area=${area} operation=${operation} owner=follow-up-feature\n`,
);
process.exitCode = 2;
