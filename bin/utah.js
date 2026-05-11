#!/usr/bin/env node
'use strict';

const path = require('path');
const args = process.argv.slice(2);

const version = require(path.join(__dirname, '..', 'package.json')).version;

console.log(`
🔷 UTAH FRAMEWORK [v${version}]
The Post-Electron Paradigm.
-----------------------------------`);

if (args.includes('--assimilate') || args.includes('--sync-core')) {
    require(path.join(__dirname, '..', 'assimilate.js'));
} else if (args[0] === 'create') {
    const projectName = args[1] || 'utah-app';
    console.log(`[ZEO-ARCHITECT] Scaffolding new Utah project: ${projectName}...`);
    console.log(
        '--> Clone the master template: https://github.com/utahisnotastate/utah-framework'
    );
    console.log(`    git clone https://github.com/utahisnotastate/utah-framework.git ${projectName}`);
} else {
    console.log('Available Commands:');
    console.log('  npx utah-framework create <name>  : Manifest a new Utah application (clone template).');
    console.log('  npx utah-framework --assimilate   : Eradicate Electron from an existing project.');
    console.log('  npx utah --assimilate             : Same (short binary alias).');
    console.log('  npx utah-framework --assimilate --sync-core  : Assimilate + copy full Rust Aegis core.');
    process.exit(args.length ? 1 : 0);
}
