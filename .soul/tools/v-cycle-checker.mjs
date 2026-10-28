#!/usr/bin/env node

/**
 * .soul V-Cycle Traceability & Gap Analysis Checker
 * 
 * Zero-dependency portable checker script. Validates:
 * 1. 9 mandatory requirement attributes in docs/specs/*.md
 * 2. Component validity against docs/archi/module-list.md
 * 3. State machine integrity (e.g. no Verified without Passed test)
 * 4. Downstream linkage to LLD and Verification tests
 * 5. Backlog.md task coverage
 */

import fs from 'node:fs';
import path from 'node:path';

const PROJECT_ROOT = process.cwd();
const SPECS_DIR = path.join(PROJECT_ROOT, 'docs', 'specs');
const MODULE_LIST_PATH = path.join(PROJECT_ROOT, 'docs', 'archi', 'module-list.md');
const BACKLOG_TASKS_DIR = path.join(PROJECT_ROOT, 'backlog', 'tasks');

const MANDATORY_ATTRIBUTES = [
  'Unique ID',
  'Text',
  'Criticality',
  'SW Component',
  'Implementation Status',
  'Verification Criteria',
  'Downstream Req',
  'Downstream Test',
  'Test Status'
];

const VALID_CRITICALITIES = new Set(['High', 'Medium', 'Low']);
const VALID_IMPL_STATUSES = new Set(['Not implemented', 'Implemented', 'Verified']);
const VALID_TEST_STATUSES = new Set(['Passed', 'Failed']);

function parseModuleList() {
  const registeredComponents = new Set();
  if (!fs.existsSync(MODULE_LIST_PATH)) {
    return registeredComponents;
  }
  const content = fs.readFileSync(MODULE_LIST_PATH, 'utf8');
  // Match `crates/...::...` or `src/...::...`
  const matches = content.matchAll(/`(?:crates|src)\/([^`:]+::[^`]+)`/g);
  for (const m of matches) {
    registeredComponents.add(m[0].replace(/`/g, ''));
  }
  return registeredComponents;
}

function parseBacklogTaskReferences() {
  const referencedSpecs = new Set();
  if (!fs.existsSync(BACKLOG_TASKS_DIR)) {
    return referencedSpecs;
  }
  const files = fs.readdirSync(BACKLOG_TASKS_DIR).filter(f => f.endsWith('.md'));
  for (const file of files) {
    const content = fs.readFileSync(path.join(BACKLOG_TASKS_DIR, file), 'utf8');
    const matches = content.matchAll(/(?:REQ-[A-Z0-9-]+|0[1-9]_[a-z0-9_]+_spec\.md)/g);
    for (const m of matches) {
      referencedSpecs.add(m[0]);
    }
  }
  return referencedSpecs;
}

function parseRequirements() {
  if (!fs.existsSync(SPECS_DIR)) {
    console.error(`Error: Specs directory not found at ${SPECS_DIR}`);
    process.exit(1);
  }

  const specFiles = fs.readdirSync(SPECS_DIR).filter(f => f.endsWith('.md') && f !== 'README.md');
  const requirements = [];
  const errors = [];

  for (const file of specFiles) {
    const filePath = path.join(SPECS_DIR, file);
    const content = fs.readFileSync(filePath, 'utf8');
    const blocks = content.split(/\n###\s+/);

    for (let i = 1; i < blocks.length; i++) {
      const block = blocks[i];
      const lines = block.split('\n');
      const header = lines[0].trim();
      const reqIdMatch = header.match(/^([A-Z0-9-]+)/);
      if (!reqIdMatch) continue;

      const req = {
        id: reqIdMatch[1],
        file,
        raw: block,
        attributes: {}
      };

      for (const line of lines) {
        const attrMatch = line.match(/^\*\s+\*\*([^:]+):\*\*\s*(.*)$/);
        if (attrMatch) {
          const key = attrMatch[1].trim();
          const val = attrMatch[2].trim().replace(/^`|`$/g, '').replace(/`/g, '');
          req.attributes[key] = val;
        }
      }

      // Validate mandatory attributes
      for (const mandatory of MANDATORY_ATTRIBUTES) {
        if (!req.attributes[mandatory]) {
          errors.push({
            reqId: req.id,
            file,
            type: 'MISSING_ATTRIBUTE',
            message: `Missing mandatory attribute: "${mandatory}"`
          });
        }
      }

      // Validate attribute domain values
      const criticality = req.attributes['Criticality'];
      if (criticality && !VALID_CRITICALITIES.has(criticality)) {
        errors.push({
          reqId: req.id,
          file,
          type: 'INVALID_CRITICALITY',
          message: `Invalid Criticality "${criticality}". Must be High, Medium, or Low.`
        });
      }

      const implStatus = req.attributes['Implementation Status'];
      if (implStatus && !VALID_IMPL_STATUSES.has(implStatus)) {
        errors.push({
          reqId: req.id,
          file,
          type: 'INVALID_IMPL_STATUS',
          message: `Invalid Implementation Status "${implStatus}".`
        });
      }

      const testStatus = req.attributes['Test Status'];
      if (testStatus && !VALID_TEST_STATUSES.has(testStatus)) {
        errors.push({
          reqId: req.id,
          file,
          type: 'INVALID_TEST_STATUS',
          message: `Invalid Test Status "${testStatus}".`
        });
      }

      // State machine logic validation
      if (implStatus === 'Verified' && testStatus !== 'Passed') {
        errors.push({
          reqId: req.id,
          file,
          type: 'ILLEGAL_STATE_COMBINATION',
          message: `Requirement marked "Verified" but Test Status is "${testStatus}" (must be "Passed").`
        });
      }

      if (implStatus === 'Not implemented' && testStatus === 'Passed') {
        errors.push({
          reqId: req.id,
          file,
          type: 'ILLEGAL_STATE_COMBINATION',
          message: `Requirement marked "Not implemented" but Test Status is "Passed".`
        });
      }

      requirements.push(req);
    }
  }

  return { requirements, errors, specFilesCount: specFiles.length };
}

function runAudit() {
  console.log('\n🔍 =================================================================');
  console.log('   .soul V-CYCLE TRACEABILITY & GAP ANALYSIS CHECKER');
  console.log('=================================================================\n');

  const { requirements, errors, specFilesCount } = parseRequirements();
  const registeredComponents = parseModuleList();
  const backlogReferences = parseBacklogTaskReferences();

  // Check SW Component linkage
  for (const req of requirements) {
    const swComp = req.attributes['SW Component'];
    if (swComp && registeredComponents.size > 0 && !registeredComponents.has(swComp)) {
      errors.push({
        reqId: req.id,
        file: req.file,
        type: 'UNREGISTERED_COMPONENT',
        message: `SW Component "${swComp}" not registered in docs/archi/module-list.md`
      });
    }
  }

  // Calculate Statistics
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

  const implCoverage = totalReqs > 0 ? (((byImplStatus['Implemented'] + byImplStatus['Verified']) / totalReqs) * 100).toFixed(1) : 0;
  const verifCoverage = totalReqs > 0 ? ((byImplStatus['Verified'] / totalReqs) * 100).toFixed(1) : 0;
  const testCoverage = totalReqs > 0 ? ((byTestStatus['Passed'] / totalReqs) * 100).toFixed(1) : 0;

  console.log(`📁 Specifications Scanned: ${specFilesCount} files`);
  console.log(`📋 Total Atomic Requirements: ${totalReqs}`);
  console.log(`   ├─ High:   ${byCriticality.High.toString().padStart(3)} (${((byCriticality.High/totalReqs)*100).toFixed(0)}%)`);
  console.log(`   ├─ Medium: ${byCriticality.Medium.toString().padStart(3)} (${((byCriticality.Medium/totalReqs)*100).toFixed(0)}%)`);
  console.log(`   └─ Low:    ${byCriticality.Low.toString().padStart(3)} (${((byCriticality.Low/totalReqs)*100).toFixed(0)}%)`);
  console.log('');
  console.log(`📊 V-Cycle Coverage Metrics:`);
  console.log(`   ├─ Implementation Rate: [${renderProgressBar(implCoverage)}] ${implCoverage}% (${byImplStatus['Implemented'] + byImplStatus['Verified']}/${totalReqs})`);
  console.log(`   ├─ Verification Rate:   [${renderProgressBar(verifCoverage)}] ${verifCoverage}% (${byImplStatus['Verified']}/${totalReqs})`);
  console.log(`   └─ Test Pass Rate:       [${renderProgressBar(testCoverage)}] ${testCoverage}% (${byTestStatus['Passed']}/${totalReqs})`);
  console.log('');

  // Report Integrity Errors / Gaps
  if (errors.length > 0) {
    console.log(`❌ Traceability Violations & Integrity Gaps Found: ${errors.length}\n`);
    for (const err of errors) {
      console.log(`   • [${err.type}] ${err.reqId} in ${err.file}`);
      console.log(`     └─ ${err.message}`);
    }
    console.log('');
  } else {
    console.log(`✅ Traceability Schema Integrity: 100% compliant (Zero schema violations)`);
  }

  // Backlog Linkage Check
  let unlinkedFromBacklog = 0;
  for (const req of requirements) {
    if (!backlogReferences.has(req.id) && !backlogReferences.has(req.file)) {
      unlinkedFromBacklog++;
    }
  }

  if (unlinkedFromBacklog > 0) {
    console.log(`⚠️  Backlog Linkage: ${totalReqs - unlinkedFromBacklog}/${totalReqs} requirements traced in backlog/tasks (${unlinkedFromBacklog} pending task assignment)`);
  } else {
    console.log(`✅ Backlog Linkage: 100% of requirements referenced in backlog tasks.`);
  }

  console.log('\n=================================================================\n');

  if (process.argv.includes('--generate-matrix')) {
    generateTraceabilityMatrix(requirements);
  }

  if (errors.length > 0 && process.argv.includes('--strict')) {
    process.exit(1);
  }
}

function renderProgressBar(percentage) {
  const totalBars = 20;
  const filledBars = Math.round((percentage / 100) * totalBars);
  return '█'.repeat(filledBars) + '░'.repeat(totalBars - filledBars);
}

function generateTraceabilityMatrix(requirements) {
  const outputPath = path.join(PROJECT_ROOT, 'docs', 'specs', 'TRACEABILITY_MATRIX.md');
  let md = '# V-Cycle Traceability Matrix\n\n';
  md += `*Auto-generated by \`.soul/tools/v-cycle-checker.mjs\` on ${new Date().toISOString().split('T')[0]}*\n\n`;
  md += '| Requirement ID | Criticality | SW Component | Impl Status | Downstream LLD | Downstream Test | Test Status |\n';
  md += '| :--- | :--- | :--- | :--- | :--- | :--- | :--- |\n';

  for (const req of requirements) {
    const a = req.attributes;
    md += `| \`${req.id}\` | ${a['Criticality']} | \`${a['SW Component']}\` | **${a['Implementation Status']}** | \`${a['Downstream Req']}\` | \`${a['Downstream Test']}\` | \`${a['Test Status']}\` |\n`;
  }

  fs.writeFileSync(outputPath, md, 'utf8');
  console.log(`📄 Generated Traceability Matrix: docs/specs/TRACEABILITY_MATRIX.md\n`);
}

runAudit();
