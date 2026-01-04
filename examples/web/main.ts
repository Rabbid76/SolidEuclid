import init from '@solid-euclid';

async function main() {
  await init();

  const app = document.getElementById('app');
  if (!app) return;

  app.innerHTML = `
    <div style="padding: 20px; font-family: Arial, sans-serif;">
      <h1>SolidEuclid Examples</h1>
      <nav>
        <ul style="list-style: none; padding: 0;">
          <li style="margin: 10px 0;">
            <a href="./platonic_solid/" style="color: #007acc; text-decoration: none; font-size: 18px;">
              Platonic Solid
            </a>
          </li>
        </ul>
      </nav>
    </div>
  `;
}

main();
