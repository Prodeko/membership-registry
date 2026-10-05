import { ReactNode } from "react";

export interface ImportColumn {
  name: string;
  required: "yes" | "no" | ReactNode;
  description: ReactNode;
}

interface Props {
  columns: ImportColumn[];
  rules: ReactNode[];
  example: string;
}

/** How to build the CSV for one import: columns, rules and an example. */
const ImportInstructions = ({ columns, rules, example }: Props) => (
  <div className="space-y-4 text-base text-foreground">
    <p>
      Upload a <strong>CSV file</strong> (comma-separated) whose first row has
      the column names below. Click <strong>Preview</strong> to check every row
      first; nothing changes until you click <strong>Confirm import</strong>.
    </p>

    <div className="overflow-x-auto rounded-md border">
      <table className="w-full text-sm">
        <thead className="bg-muted text-left">
          <tr>
            <th className="px-3 py-2 font-medium">Column</th>
            <th className="px-3 py-2 font-medium">Required</th>
            <th className="px-3 py-2 font-medium">What to put in it</th>
          </tr>
        </thead>
        <tbody>
          {columns.map((c) => (
            <tr key={c.name} className="border-t align-top">
              <td className="px-3 py-2 font-mono whitespace-nowrap">
                {c.name}
              </td>
              <td className="px-3 py-2 whitespace-nowrap">{c.required}</td>
              <td className="px-3 py-2">{c.description}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>

    <ul className="list-disc space-y-1 pl-5">
      {rules.map((rule, i) => (
        <li key={i}>{rule}</li>
      ))}
    </ul>

    <div className="space-y-1">
      <p className="text-sm font-medium">Example</p>
      <pre className="overflow-x-auto rounded-md bg-muted px-3 py-2 text-sm">
        {example}
      </pre>
    </div>
  </div>
);

export default ImportInstructions;
