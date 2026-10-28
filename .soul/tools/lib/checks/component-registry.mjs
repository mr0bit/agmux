export const componentRegistryCheck = {
  id: 'CHECK-04-COMPONENT-REGISTRY',
  name: 'Module Root-of-Trust Check',
  description: 'Verifies that every requirement is assigned to a component officially registered in docs/archi/module-list.md.',
  run({ requirements, registeredComponents }) {
    const errors = [];
    if (!registeredComponents || registeredComponents.size === 0) {
      return errors;
    }

    for (const req of requirements) {
      const swComp = req.attributes['SW Component'];
      if (swComp && !registeredComponents.has(swComp)) {
        errors.push({
          reqId: req.id,
          file: req.file,
          type: 'UNREGISTERED_COMPONENT',
          message: `SW Component "${swComp}" not registered in docs/archi/module-list.md`
        });
      }
    }
    return errors;
  }
};
