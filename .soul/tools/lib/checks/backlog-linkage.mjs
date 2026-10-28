export const backlogLinkageCheck = {
  id: 'CHECK-05-BACKLOG-LINKAGE',
  name: 'Backlog Task Allocation Check',
  description: 'Audits whether requirements are allocated to active tasks in .backlog/tasks/ (or backlog/tasks/).',
  run({ requirements, backlogReferences }) {
    let unlinkedCount = 0;
    const unlinkedReqs = [];

    for (const req of requirements) {
      if (!backlogReferences.has(req.id) && !backlogReferences.has(req.file)) {
        unlinkedCount++;
        unlinkedReqs.push(req.id);
      }
    }

    return {
      total: requirements.length,
      linked: requirements.length - unlinkedCount,
      unlinkedCount,
      unlinkedReqs
    };
  }
};
