// Sends a result to bench.py's server. Same origin, so no CORS preflight.
function benchReport(kind, data) {
  const body = JSON.stringify(Object.assign({ kind, page: location.pathname, query: location.search }, data));
  fetch("/report", { method: "POST", body, keepalive: true });
}
