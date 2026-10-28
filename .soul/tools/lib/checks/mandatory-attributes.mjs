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

export const mandatoryAttributesCheck = {
  id: 'CHECK-01-MANDATORY-ATTRIBUTES',
  name: 'Mandatory 9-Attribute Verification',
  description: 'Ensures every atomic requirement defines all 9 required traceability fields.',
  run({ requirements }) {
    const errors = [];
    for (const req of requirements) {
      for (const attr of MANDATORY_ATTRIBUTES) {
        if (!req.attributes[attr]) {
          errors.push({
            reqId: req.id,
            file: req.file,
            type: 'MISSING_ATTRIBUTE',
            message: `Missing mandatory attribute: "${attr}"`
          });
        }
      }
    }
    return errors;
  }
};
