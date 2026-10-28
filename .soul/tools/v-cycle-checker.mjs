#!/usr/bin/env node

/**
 * .soul V-Cycle Traceability & Gap Analysis Checker
 * 
 * Modular, zero-dependency architecture:
 * - Pluggable check rules in .soul/tools/lib/checks/
 * - Pluggable reporters in .soul/tools/lib/reporter-*.mjs
 */

import fs from 'node:fs';
import path from 'node:path';
import { parseRequirements, parseModuleList, parseBacklogTaskReferences } from './lib/parser.mjs';
import { runAllChecks } from './lib/checks/index.mjs';
import { renderTerminalReport } from './lib/reporter-terminal.mjs';
import { generateTraceabilityMatrix } from './lib/reporter-markdown.mjs';
import { generateInteractiveHtmlReport } from './lib/reporter-html.mjs';

const PROJECT_ROOT = process.cwd();
const SPECS_DIR = path.join(PROJECT_ROOT, 'docs', 'specs');
const MODULE_LIST_PATH = path.join(PROJECT_ROOT, 'docs', 'archi', 'module-list.md');
const BACKLOG_TASKS_DIR = fs.existsSync(path.join(PROJECT_ROOT, '.backlog', 'tasks'))
  ? path.join(PROJECT_ROOT, '.backlog', 'tasks')
  : path.join(PROJECT_ROOT, 'backlog', 'tasks');

function main() {
  const { requirements, specFilesCount } = parseRequirements(SPECS_DIR);
  const registeredComponents = parseModuleList(MODULE_LIST_PATH);
  const backlogReferences = parseBacklogTaskReferences(BACKLOG_TASKS_DIR);

  // Context passed to all modular checks
  const context = {
    requirements,
    registeredComponents,
    backlogReferences
  };

  // Run all registered pluggable checks
  const { errors, backlogAudit } = runAllChecks(context);

  // Aggregate stats
  const totalReqs = requirements.length;
  const byCriticality = { High: 0, Medium: 0, Low: 0 };
  const byImplStatus = { 'Not implemented': 0, 'Implemented': 0, 'Verified': 0 };
  const byTestStatus = { Passed: 0, Failed: 0 };

  for (const req of requirements) {
    const crit = req.attributes['Criticality'];
    if (crit && byCriticality[crit] !== undefined) byCriticality[crit]++;

    const impl = req.attributes['Implementation Status'];
    if (impl && byImplStatus[impl] !== undefined) byImplStatus[impl]++;

    const test = req.attributes['Test Status'];
    if (test && byTestStatus[test] !== undefined) byTestStatus[test]++;
  }

  // 1. Terminal Reporter
  renderTerminalReport({
    specFilesCount,
    totalReqs,
    byCriticality,
    byImplStatus,
    byTestStatus,
    errors,
    backlogAudit
  });

  // 2. Markdown Matrix Reporter (opt-in or auto)
  if (process.argv.includes('--generate-matrix')) {
    generateTraceabilityMatrix(PROJECT_ROOT, requirements);
  }

  // 3. Interactive HTML Dashboard Reporter (opt-in or auto)
  if (process.argv.includes('--html') || process.argv.includes('--report')) {
    generateInteractiveHtmlReport(PROJECT_ROOT, {
      requirements,
      totalReqs,
      byCriticality,
      byImplStatus,
      byTestStatus,
      errors
    });
  }

  // 4. Strict CI Gate Enforcement
  if (errors.length > 0 && process.argv.includes('--strict')) {
    process.exit(1);
  }
}

main();
