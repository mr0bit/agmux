export function renderTerminalReport({
  specFilesCount,
  totalReqs,
  byCriticality,
  byImplStatus,
  byTestStatus,
  errors,
  backlogAudit
}) {
  const implCoverage = totalReqs > 0 ? (((byImplStatus['Implemented'] + byImplStatus['Verified']) / totalReqs) * 100).toFixed(1) : 0;
  const verifCoverage = totalReqs > 0 ? ((byImplStatus['Verified'] / totalReqs) * 100).toFixed(1) : 0;
  const testCoverage = totalReqs > 0 ? ((byTestStatus['Passed'] / totalReqs) * 100).toFixed(1) : 0;

  console.log('\n🔍 =================================================================');
  console.log('   .soul V-CYCLE TRACEABILITY & GAP ANALYSIS CHECKER');
  console.log('=================================================================\n');

  console.log(`📁 Specifications Scanned: ${specFilesCount} files`);
  console.log(`📋 Total Atomic Requirements: ${totalReqs}`);
  console.log(`   ├─ High:   ${byCriticality.High.toString().padStart(3)} (${((byCriticality.High / totalReqs) * 100).toFixed(0)}%)`);
  console.log(`   ├─ Medium: ${byCriticality.Medium.toString().padStart(3)} (${((byCriticality.Medium / totalReqs) * 100).toFixed(0)}%)`);
  console.log(`   └─ Low:    ${byCriticality.Low.toString().padStart(3)} (${((byCriticality.Low / totalReqs) * 100).toFixed(0)}%)`);
  console.log('');
  console.log(`📊 V-Cycle Coverage Metrics:`);
  console.log(`   ├─ Implementation Rate: [${renderProgressBar(implCoverage)}] ${implCoverage}% (${byImplStatus['Implemented'] + byImplStatus['Verified']}/${totalReqs})`);
  console.log(`   ├─ Verification Rate:   [${renderProgressBar(verifCoverage)}] ${verifCoverage}% (${byImplStatus['Verified']}/${totalReqs})`);
  console.log(`   └─ Test Pass Rate:       [${renderProgressBar(testCoverage)}] ${testCoverage}% (${byTestStatus['Passed']}/${totalReqs})`);
  console.log('');

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

  if (backlogAudit.unlinkedCount > 0) {
    console.log(`⚠️  Backlog Linkage: ${backlogAudit.linked}/${totalReqs} requirements traced in backlog/tasks (${backlogAudit.unlinkedCount} pending task assignment)`);
  } else {
    console.log(`✅ Backlog Linkage: 100% of requirements referenced in backlog tasks.`);
  }

  console.log('\n=================================================================\n');
}

function renderProgressBar(percentage) {
  const totalBars = 20;
  const filledBars = Math.round((percentage / 100) * totalBars);
  return '█'.repeat(filledBars) + '░'.repeat(totalBars - filledBars);
}
