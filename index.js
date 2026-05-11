'use strict';

/**
 * Programmatic entry for `require('utah-framework')` after npm install.
 * The primary surface is the CLI: `utah-framework`, `utah`.
 */
const pkg = require('./package.json');

module.exports = {
    version: pkg.version,
    name: pkg.name,
};
