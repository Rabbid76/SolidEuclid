import init, { hello_wasm } from '@solid-euclid';

async function main() {
  await init();
  const app = document.getElementById('app');
  if (app) {
    app.textContent = hello_wasm();
  }
}

main();
