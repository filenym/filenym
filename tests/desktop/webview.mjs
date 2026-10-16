export async function connect(port = 9223) {
  const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
  const candidates = targets.filter((t) => {
    if (t.type !== "page") return false;
    const origin = new URL(t.url).origin;
    return origin === "http://localhost:5173" || origin === "http://tauri.localhost";
  });
  if (candidates.length !== 1) throw new Error("Expected exactly one isolated FileNym target");
  const target = candidates[0];
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", reject, { once: true });
  });
  let id = 0;
  const waiting = new Map();
  ws.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && waiting.has(message.id)) {
      const { resolve, reject, timer } = waiting.get(message.id);
      clearTimeout(timer);
      waiting.delete(message.id);
      if (message.error) reject(new Error(JSON.stringify(message.error)));
      else resolve(message.result);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const number = ++id;
      const timer = setTimeout(() => {
        waiting.delete(number);
        reject(new Error(`Timeout: ${method}`));
      }, 20000);
      waiting.set(number, { resolve, reject, timer });
      ws.send(JSON.stringify({ id: number, method, params }));
    });
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  return { send, evaluate, close: () => ws.close() };
}
