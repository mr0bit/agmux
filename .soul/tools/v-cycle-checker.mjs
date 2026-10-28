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

  const specFiles = fs.readdirSync(SPECS_DIR).filter(f => f.endsWith('.md') && f !== 'README.md' && f !== 'TRACEABILITY_MATRIX.md');
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

  if (process.argv.includes('--html') || process.argv.includes('--report')) {
    generateInteractiveHtmlReport(requirements, totalReqs, byCriticality, byImplStatus, byTestStatus, errors, unlinkedFromBacklog);
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

function generateInteractiveHtmlReport(requirements, totalReqs, byCriticality, byImplStatus, byTestStatus, errors, unlinkedFromBacklog) {
  const outputPath = path.join(PROJECT_ROOT, 'docs', 'specs', 'traceability_report.html');
  const now = new Date().toISOString().replace('T', ' ').substring(0, 19) + ' UTC';

  const implCount = byImplStatus['Implemented'] + byImplStatus['Verified'];
  const implPercent = totalReqs > 0 ? ((implCount / totalReqs) * 100).toFixed(1) : '0.0';
  const verifCount = byImplStatus['Verified'];
  const verifPercent = totalReqs > 0 ? ((verifCount / totalReqs) * 100).toFixed(1) : '0.0';
  const testCount = byTestStatus['Passed'];
  const testPercent = totalReqs > 0 ? ((testCount / totalReqs) * 100).toFixed(1) : '0.0';

  const reqDataJson = JSON.stringify(requirements.map(r => ({
    id: r.id,
    file: r.file,
    criticality: r.attributes['Criticality'] || 'Unknown',
    component: r.attributes['SW Component'] || 'Unassigned',
    implStatus: r.attributes['Implementation Status'] || 'Not implemented',
    testStatus: r.attributes['Test Status'] || 'Failed',
    downstreamReq: r.attributes['Downstream Req'] || 'TBD',
    downstreamTest: r.attributes['Downstream Test'] || 'TBD',
    text: r.attributes['Text'] || '',
    verificationCriteria: r.attributes['Verification Criteria'] || ''
  })));

  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>V-Cycle Traceability & Gap Analysis Dashboard</title>
  <style>
    :root {
      --bg: #0d1117;
      --card-bg: #161b22;
      --border: #30363d;
      --text: #c9d1d9;
      --text-muted: #8b949e;
      --accent: #58a6ff;
      --success: #3fb950;
      --warning: #d29922;
      --danger: #f85149;
      --purple: #bc8cff;
      --font: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background-color: var(--bg);
      color: var(--text);
      font-family: var(--font);
      line-height: 1.5;
      padding: 24px;
    }

    header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 24px;
      padding-bottom: 16px;
      border-bottom: 1px solid var(--border);
    }

    h1 { font-size: 24px; font-weight: 600; color: #f0f6fc; }
    .subtitle { font-size: 13px; color: var(--text-muted); margin-top: 4px; }

    .header-actions {
      display: flex;
      gap: 10px;
    }

    .btn {
      background-color: #21262d;
      color: var(--text);
      border: 1px solid var(--border);
      padding: 6px 14px;
      border-radius: 6px;
      font-size: 13px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.15s ease;
      display: inline-flex;
      align-items: center;
      gap: 6px;
    }
    .btn:hover { background-color: #30363d; border-color: #8b949e; }
    .btn.active { background-color: #1f6feb; border-color: #388bfd; color: #fff; }
    .btn-danger { color: #ff7b72; }
    .btn-danger.active { background-color: #b62324; color: #fff; border-color: #da3633; }

    .kpi-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 16px;
      margin-bottom: 24px;
    }

    .kpi-card {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 16px;
    }

    .kpi-title { font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-muted); }
    .kpi-value { font-size: 28px; font-weight: 700; color: #f0f6fc; margin: 8px 0; }
    .kpi-sub { font-size: 12px; color: var(--text-muted); }

    .progress-bar {
      height: 6px;
      background: #21262d;
      border-radius: 3px;
      overflow: hidden;
      margin-top: 8px;
    }
    .progress-fill { height: 100%; border-radius: 3px; transition: width 0.3s ease; }
    .fill-blue { background: var(--accent); }
    .fill-green { background: var(--success); }
    .fill-orange { background: var(--warning); }

    .toolbar {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 16px;
      margin-bottom: 20px;
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      align-items: center;
    }

    .search-box {
      flex: 1;
      min-width: 250px;
      position: relative;
    }

    input[type="text"], select {
      background: #0d1117;
      border: 1px solid var(--border);
      color: var(--text);
      padding: 7px 12px;
      border-radius: 6px;
      font-size: 13px;
      outline: none;
      width: 100%;
    }
    input[type="text"]:focus, select:focus { border-color: var(--accent); }

    .filter-group {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .filter-group label { font-size: 12px; color: var(--text-muted); font-weight: 500; }
    select { width: auto; min-width: 130px; cursor: pointer; }

    .table-container {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      overflow-x: auto;
    }

    table {
      width: 100%;
      border-collapse: collapse;
      text-align: left;
      font-size: 13px;
    }

    th {
      background: #1c2128;
      color: #f0f6fc;
      padding: 12px 14px;
      border-bottom: 1px solid var(--border);
      font-weight: 600;
      white-space: nowrap;
    }

    td {
      padding: 12px 14px;
      border-bottom: 1px solid var(--border);
      color: var(--text);
      vertical-align: middle;
    }

    tbody tr { cursor: pointer; transition: background 0.15s ease; }
    tbody tr:hover { background: #1c2128; }
    tbody tr.expanded { background: #1c2128; }

    .badge {
      display: inline-block;
      padding: 2px 8px;
      border-radius: 12px;
      font-size: 11px;
      font-weight: 600;
      line-height: 1.4;
    }
    .badge-high { background: rgba(248, 81, 73, 0.15); color: #ff7b72; border: 1px solid rgba(248, 81, 73, 0.4); }
    .badge-med { background: rgba(210, 153, 34, 0.15); color: #d29922; border: 1px solid rgba(210, 153, 34, 0.4); }
    .badge-low { background: rgba(139, 148, 158, 0.15); color: #8b949e; border: 1px solid rgba(139, 148, 158, 0.4); }

    .badge-verified { background: rgba(63, 185, 80, 0.15); color: #56d364; border: 1px solid rgba(63, 185, 80, 0.4); }
    .badge-impl { background: rgba(88, 166, 255, 0.15); color: #79c0ff; border: 1px solid rgba(88, 166, 255, 0.4); }
    .badge-notimpl { background: rgba(139, 148, 158, 0.15); color: #8b949e; border: 1px solid rgba(139, 148, 158, 0.4); }

    .badge-pass { background: rgba(63, 185, 80, 0.15); color: #56d364; border: 1px solid rgba(63, 185, 80, 0.4); }
    .badge-fail { background: rgba(248, 81, 73, 0.15); color: #ff7b72; border: 1px solid rgba(248, 81, 73, 0.4); }

    .code-pill {
      font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
      font-size: 12px;
      background: #0d1117;
      padding: 2px 6px;
      border-radius: 4px;
      border: 1px solid #30363d;
      color: #79c0ff;
    }

    .detail-row {
      display: none;
      background: #0d1117;
    }
    .detail-row.open { display: table-row; }
    .detail-content {
      padding: 16px 20px;
      border-bottom: 1px solid var(--border);
    }
    .detail-section { margin-bottom: 12px; }
    .detail-section:last-child { margin-bottom: 0; }
    .detail-label { font-size: 11px; text-transform: uppercase; font-weight: 700; color: var(--text-muted); margin-bottom: 4px; }
    .detail-body { font-size: 13px; color: #f0f6fc; line-height: 1.6; }

    .results-count {
      font-size: 13px;
      color: var(--text-muted);
      margin-left: auto;
    }

    @media print {
      body { background: white; color: black; padding: 0; }
      .toolbar, .header-actions { display: none; }
      .kpi-card, .table-container { border: 1px solid #ccc; background: white; color: black; }
      th { background: #f0f0f0; color: black; }
      td { color: black; }
      .code-pill { border: 1px solid #ddd; color: black; background: #f9f9f9; }
    }
  </style>
</head>
<body>

  <header>
    <div>
      <h1>V-Cycle Traceability & Gap Analysis Dashboard</h1>
      <div class="subtitle">Generated by <code>.soul/tools/v-cycle-checker.mjs</code> &bull; ${now}</div>
    </div>
    <div class="header-actions">
      <button class="btn btn-danger" id="gapFilterBtn" onclick="toggleGapFilter()">⚠️ Show Only Gaps</button>
      <button class="btn" onclick="exportCsv()">📥 Export CSV</button>
      <button class="btn" onclick="window.print()">🖨️ Print / PDF</button>
    </div>
  </header>

  <div class="kpi-grid">
    <div class="kpi-card">
      <div class="kpi-title">Total Requirements</div>
      <div class="kpi-value">${totalReqs}</div>
      <div class="kpi-sub">${byCriticality.High} High &bull; ${byCriticality.Medium} Medium &bull; ${byCriticality.Low} Low</div>
    </div>

    <div class="kpi-card">
      <div class="kpi-title">Implementation Rate</div>
      <div class="kpi-value">${implPercent}%</div>
      <div class="kpi-sub">${implCount} of ${totalReqs} implemented</div>
      <div class="progress-bar"><div class="progress-fill fill-blue" style="width: ${implPercent}%"></div></div>
    </div>

    <div class="kpi-card">
      <div class="kpi-title">Verification Rate</div>
      <div class="kpi-value">${verifPercent}%</div>
      <div class="kpi-sub">${verifCount} of ${totalReqs} verified</div>
      <div class="progress-bar"><div class="progress-fill fill-green" style="width: ${verifPercent}%"></div></div>
    </div>

    <div class="kpi-card">
      <div class="kpi-title">Test Pass Rate</div>
      <div class="kpi-value">${testPercent}%</div>
      <div class="kpi-sub">${testCount} passed &bull; ${totalReqs - testCount} pending/failed</div>
      <div class="progress-bar"><div class="progress-fill fill-orange" style="width: ${testPercent}%"></div></div>
    </div>

    <div class="kpi-card">
      <div class="kpi-title">Schema & Integrity</div>
      <div class="kpi-value" style="color: ${errors.length === 0 ? 'var(--success)' : 'var(--danger)'};">
        ${errors.length === 0 ? '100%' : errors.length + ' Gaps'}
      </div>
      <div class="kpi-sub">${errors.length === 0 ? 'Zero schema violations' : errors.length + ' integrity warnings found'}</div>
    </div>
  </div>

  <div class="toolbar">
    <div class="search-box">
      <input type="text" id="searchInput" placeholder="Search by ID, text, component, or test..." oninput="applyFilters()">
    </div>

    <div class="filter-group">
      <label for="criticalityFilter">Criticality:</label>
      <select id="criticalityFilter" onchange="applyFilters()">
        <option value="ALL">All</option>
        <option value="High">High</option>
        <option value="Medium">Medium</option>
        <option value="Low">Low</option>
      </select>
    </div>

    <div class="filter-group">
      <label for="implFilter">Implementation:</label>
      <select id="implFilter" onchange="applyFilters()">
        <option value="ALL">All</option>
        <option value="Not implemented">Not implemented</option>
        <option value="Implemented">Implemented</option>
        <option value="Verified">Verified</option>
      </select>
    </div>

    <div class="filter-group">
      <label for="testFilter">Test Status:</label>
      <select id="testFilter" onchange="applyFilters()">
        <option value="ALL">All</option>
        <option value="Passed">Passed</option>
        <option value="Failed">Failed</option>
      </select>
    </div>

    <div class="filter-group">
      <label for="specFilter">Specification:</label>
      <select id="specFilter" onchange="applyFilters()">
        <option value="ALL">All Files</option>
      </select>
    </div>

    <button class="btn" onclick="resetFilters()">Reset</button>

    <div class="results-count" id="resultsCount">Showing 0 of 0</div>
  </div>

  <div class="table-container">
    <table>
      <thead>
        <tr>
          <th>Requirement ID</th>
          <th>Criticality</th>
          <th>SW Component</th>
          <th>Implementation</th>
          <th>Downstream Req</th>
          <th>Downstream Test</th>
          <th>Test Status</th>
        </tr>
      </thead>
      <tbody id="tableBody">
        <!-- Rendered by JavaScript -->
      </tbody>
    </table>
  </div>

  <script>
    const requirements = ${reqDataJson};
    let onlyGapsMode = false;

    function init() {
      // Populate Spec Files dropdown
      const files = [...new Set(requirements.map(r => r.file))].sort();
      const specSelect = document.getElementById('specFilter');
      files.forEach(f => {
        const opt = document.createElement('option');
        opt.value = f;
        opt.textContent = f;
        specSelect.appendChild(opt);
      });

      renderTable(requirements);
    }

    function toggleGapFilter() {
      onlyGapsMode = !onlyGapsMode;
      const btn = document.getElementById('gapFilterBtn');
      btn.classList.toggle('active', onlyGapsMode);
      applyFilters();
    }

    function resetFilters() {
      document.getElementById('searchInput').value = '';
      document.getElementById('criticalityFilter').value = 'ALL';
      document.getElementById('implFilter').value = 'ALL';
      document.getElementById('testFilter').value = 'ALL';
      document.getElementById('specFilter').value = 'ALL';
      onlyGapsMode = false;
      document.getElementById('gapFilterBtn').classList.remove('active');
      applyFilters();
    }

    function applyFilters() {
      const q = document.getElementById('searchInput').value.toLowerCase().trim();
      const crit = document.getElementById('criticalityFilter').value;
      const impl = document.getElementById('implFilter').value;
      const test = document.getElementById('testFilter').value;
      const spec = document.getElementById('specFilter').value;

      const filtered = requirements.filter(r => {
        if (onlyGapsMode && r.implStatus === 'Verified' && r.testStatus === 'Passed') return false;
        if (crit !== 'ALL' && r.criticality !== crit) return false;
        if (impl !== 'ALL' && r.implStatus !== impl) return false;
        if (test !== 'ALL' && r.testStatus !== test) return false;
        if (spec !== 'ALL' && r.file !== spec) return false;

        if (q) {
          const match = r.id.toLowerCase().includes(q) ||
                        r.text.toLowerCase().includes(q) ||
                        r.component.toLowerCase().includes(q) ||
                        r.downstreamReq.toLowerCase().includes(q) ||
                        r.downstreamTest.toLowerCase().includes(q) ||
                        r.verificationCriteria.toLowerCase().includes(q);
          if (!match) return false;
        }

        return true;
      });

      renderTable(filtered);
    }

    function renderTable(items) {
      const tbody = document.getElementById('tableBody');
      tbody.innerHTML = '';

      document.getElementById('resultsCount').textContent = \`Showing \${items.length} of \${requirements.length} requirements\`;

      if (items.length === 0) {
        tbody.innerHTML = '<tr><td colspan="7" style="text-align: center; padding: 32px; color: var(--text-muted);">No requirements match the current filter criteria.</td></tr>';
        return;
      }

      items.forEach((r, idx) => {
        const tr = document.createElement('tr');
        tr.onclick = () => toggleDetail(idx);

        const critBadge = r.criticality === 'High' ? 'badge-high' : r.criticality === 'Medium' ? 'badge-med' : 'badge-low';
        const implBadge = r.implStatus === 'Verified' ? 'badge-verified' : r.implStatus === 'Implemented' ? 'badge-impl' : 'badge-notimpl';
        const testBadge = r.testStatus === 'Passed' ? 'badge-pass' : 'badge-fail';

        tr.innerHTML = \`
          <td><strong style="color: #f0f6fc;">\${r.id}</strong></td>
          <td><span class="badge \${critBadge}">\${r.criticality}</span></td>
          <td><span class="code-pill">\${r.component}</span></td>
          <td><span class="badge \${implBadge}">\${r.implStatus}</span></td>
          <td><span class="code-pill">\${r.downstreamReq}</span></td>
          <td><span class="code-pill">\${r.downstreamTest}</span></td>
          <td><span class="badge \${testBadge}">\${r.testStatus}</span></td>
        \`;

        const detailTr = document.createElement('tr');
        detailTr.id = 'detail-' + idx;
        detailTr.className = 'detail-row';
        detailTr.innerHTML = \`
          <td colspan="7" style="padding: 0;">
            <div class="detail-content">
              <div class="detail-section">
                <div class="detail-label">User Story Requirement</div>
                <div class="detail-body">\${escapeHtml(r.text)}</div>
              </div>
              <div class="detail-section">
                <div class="detail-label">Verification Criteria</div>
                <div class="detail-body">\${escapeHtml(r.verificationCriteria)}</div>
              </div>
              <div class="detail-section" style="font-size: 11px; color: var(--text-muted);">
                Source File: <code>docs/specs/\${r.file}</code> &bull; Component: <code>\${r.component}</code>
              </div>
            </div>
          </td>
        \`;

        tbody.appendChild(tr);
        tbody.appendChild(detailTr);
      });
    }

    function toggleDetail(idx) {
      const row = document.getElementById('detail-' + idx);
      row.classList.toggle('open');
    }

    function escapeHtml(text) {
      const map = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;' };
      return text.replace(/[&<>"']/g, m => map[m]);
    }

    function exportCsv() {
      let csv = 'Requirement ID,Criticality,SW Component,Implementation Status,Downstream Req,Downstream Test,Test Status,Spec File\\n';
      requirements.forEach(r => {
        csv += \`"\${r.id}","\${r.criticality}","\${r.component}","\${r.implStatus}","\${r.downstreamReq}","\${r.downstreamTest}","\${r.testStatus}","\${r.file}"\\n\`;
      });
      const blob = new Blob([csv], { type: 'text/csv' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'traceability_matrix.csv';
      a.click();
      URL.revokeObjectURL(url);
    }

    init();
  </script>
</body>
</html>
`;

  fs.writeFileSync(outputPath, html, 'utf8');
  console.log(`🌐 Generated Interactive HTML Dashboard: docs/specs/traceability_report.html\n`);
}


runAudit();
