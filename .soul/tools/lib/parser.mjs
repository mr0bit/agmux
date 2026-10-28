import fs from 'node:fs';
import path from 'node:path';

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

export function parseModuleList(moduleListPath) {
  const registeredComponents = new Set();
  if (!fs.existsSync(moduleListPath)) {
    return registeredComponents;
  }
  const content = fs.readFileSync(moduleListPath, 'utf8');
  const matches = content.matchAll(/`(?:crates|src)\/([^`:]+::[^`]+)`/g);
  for (const m of matches) {
    registeredComponents.add(m[0].replace(/`/g, ''));
  }
  return registeredComponents;
}

export function parseBacklogTaskReferences(backlogTasksDir) {
  const referencedSpecs = new Set();
  if (!fs.existsSync(backlogTasksDir)) {
    return referencedSpecs;
  }
  const files = fs.readdirSync(backlogTasksDir).filter(f => f.endsWith('.md'));
  for (const file of files) {
    const content = fs.readFileSync(path.join(backlogTasksDir, file), 'utf8');
    const matches = content.matchAll(/(?:SWRS-[A-Z0-9-]+|REQ-[A-Z0-9-]+|[0-9]{2}_[a-z0-9_]+_spec\.md)/g);
    for (const m of matches) {
      referencedSpecs.add(m[0]);
    }
  }
  return referencedSpecs;
}

export function parseRequirements(specsDir) {
  if (!fs.existsSync(specsDir)) {
    throw new Error(`Specs directory not found at ${specsDir}`);
  }

  const specFiles = fs.readdirSync(specsDir).filter(f => 
    f.endsWith('.md') && 
    f !== 'README.md' && 
    f !== 'TRACEABILITY_MATRIX.md'
  );

  const requirements = [];

  for (const file of specFiles) {
    const filePath = path.join(specsDir, file);
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

      requirements.push(req);
    }
  }

  return { requirements, specFilesCount: specFiles.length };
}
