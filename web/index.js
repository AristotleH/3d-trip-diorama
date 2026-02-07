import init, { init_diorama } from './pkg/diorama_app.js';

async function run() {
    await init();
    await init_diorama();
}

run().catch(err => console.error("Failed to start diorama:", err));
