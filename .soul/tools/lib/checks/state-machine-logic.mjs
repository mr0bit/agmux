export const stateMachineLogicCheck = {
  id: 'CHECK-03-STATE-MACHINE-LOGIC',
  name: 'State Transition Integrity Check',
  description: 'Prohibits illegal lifecycle states (e.g. Verified without Passed test).',
  run({ requirements }) {
    const errors = [];
    for (const req of requirements) {
      const impl = req.attributes['Implementation Status'];
      const test = req.attributes['Test Status'];

      if (impl === 'Verified' && test !== 'Passed') {
        errors.push({
          reqId: req.id,
          file: req.file,
          type: 'ILLEGAL_STATE_COMBINATION',
          message: `Requirement marked "Verified" but Test Status is "${test}" (must be "Passed").`
        });
      }

      if (impl === 'Not implemented' && test === 'Passed') {
        errors.push({
          reqId: req.id,
          file: req.file,
          type: 'ILLEGAL_STATE_COMBINATION',
          message: `Requirement marked "Not implemented" but Test Status is "Passed".`
        });
      }
    }
    return errors;
  }
};
