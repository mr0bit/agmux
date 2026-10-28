const VALID_CRITICALITIES = new Set(['High', 'Medium', 'Low']);
const VALID_IMPL_STATUSES = new Set(['Not implemented', 'Implemented', 'Verified']);
const VALID_TEST_STATUSES = new Set(['Passed', 'Failed']);

export const allowedValuesCheck = {
  id: 'CHECK-02-ALLOWED-VALUES',
  name: 'Enumerated Value Domain Check',
  description: 'Validates that Criticality, Implementation Status, and Test Status conform to the allowed schema.',
  run({ requirements }) {
    const errors = [];
    for (const req of requirements) {
      const crit = req.attributes['Criticality'];
      if (crit && !VALID_CRITICALITIES.has(crit)) {
        errors.push({
          reqId: req.id,
          file: req.file,
          type: 'INVALID_CRITICALITY',
          message: `Invalid Criticality "${crit}". Must be High, Medium, or Low.`
        });
      }

      const impl = req.attributes['Implementation Status'];
      if (impl && !VALID_IMPL_STATUSES.has(impl)) {
        errors.push({
          reqId: req.id,
          file: req.file,
          type: 'INVALID_IMPL_STATUS',
          message: `Invalid Implementation Status "${impl}".`
        });
      }

      const test = req.attributes['Test Status'];
      if (test && !VALID_TEST_STATUSES.has(test)) {
        errors.push({
          reqId: req.id,
          file: req.file,
          type: 'INVALID_TEST_STATUS',
          message: `Invalid Test Status "${test}".`
        });
      }
    }
    return errors;
  }
};
