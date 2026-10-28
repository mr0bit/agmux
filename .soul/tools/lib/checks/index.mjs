import { mandatoryAttributesCheck } from './mandatory-attributes.mjs';
import { allowedValuesCheck } from './allowed-values.mjs';
import { stateMachineLogicCheck } from './state-machine-logic.mjs';
import { componentRegistryCheck } from './component-registry.mjs';
import { backlogLinkageCheck } from './backlog-linkage.mjs';

/**
 * Built-in Checker Rule Registry.
 * Extensible for new checks by simply adding rules to this list.
 */
export const activeChecks = [
  mandatoryAttributesCheck,
  allowedValuesCheck,
  stateMachineLogicCheck,
  componentRegistryCheck
];

export function runAllChecks(context) {
  const allErrors = [];

  for (const check of activeChecks) {
    const errors = check.run(context);
    if (Array.isArray(errors)) {
      allErrors.push(...errors);
    }
  }

  const backlogAudit = backlogLinkageCheck.run(context);

  return {
    errors: allErrors,
    backlogAudit
  };
}
