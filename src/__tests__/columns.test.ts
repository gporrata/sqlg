import { describe, expect, it } from "vitest";
import { columnGroups } from "../lib/columns";
import { typeTone, type SchemaSnapshot } from "../lib/schema";

const driverTrip: SchemaSnapshot = {
  lowerCaseTableNames: 0,
  schemas: [
    {
      name: "fleet",
      tables: [
        {
          name: "driver",
          columns: [
            { name: "id", dataType: "int", columnType: "int", columnKey: "PRI", foreignKey: false },
            { name: "name", dataType: "varchar", columnType: "varchar(40)", columnKey: "", foreignKey: false },
          ],
        },
        {
          name: "trip",
          columns: [
            { name: "id", dataType: "int", columnType: "int", columnKey: "PRI", foreignKey: false },
            { name: "status", dataType: "enum", columnType: "enum('a','b')", columnKey: "", foreignKey: false },
            { name: "started_at", dataType: "datetime", columnType: "datetime", columnKey: "", foreignKey: false },
          ],
        },
      ],
    },
  ],
};

describe("column type tones", () => {
  it("colors mysql types by kind", () => {
    expect(typeTone("INT")).toBe("num");
    expect(typeTone("bigint unsigned")).toBe("num");
    expect(typeTone("decimal(8,2)")).toBe("num");
    expect(typeTone("varchar(255)")).toBe("text");
    expect(typeTone("JSON")).toBe("text");
    expect(typeTone("datetime")).toBe("time");
    expect(typeTone("timestamp")).toBe("time");
    expect(typeTone("tinyint(1)")).toBe("enum");
    expect(typeTone("enum('a','b')")).toBe("enum");
    expect(typeTone("geometry")).toBe("other");
  });
});

describe("column groups", () => {
  it("groups qualified columns by table alias", () => {
    const groups = columnGroups(
      "select d.id, d.name, t.status from driver d join trip t on t.driver_id = d.id",
      [
        { name: "id", typeName: "INT" },
        { name: "name", typeName: "VARCHAR" },
        { name: "status", typeName: "ENUM" },
      ],
    );
    expect(groups.map((group) => group.alias)).toEqual(["d", "t"]);
    expect(groups[0]?.columns.map((column) => column.name)).toEqual(["id", "name"]);
    expect(groups[1]?.columns.map((column) => column.name)).toEqual(["status"]);
  });

  it("keeps aliases across comments, strings, and backticks", () => {
    const groups = columnGroups(
      "select d.`id`, 'a, b' as label /* note */ from `driver` d -- tail\n where d.id > 1",
      [
        { name: "id", typeName: "INT" },
        { name: "label", typeName: "VARCHAR" },
      ],
    );
    expect(groups.map((group) => [group.alias, group.columns.map((column) => column.name)])).toEqual([
      ["d", ["id"]],
      ["expr", ["label"]],
    ]);
  });

  it("puts an unqualified list under the only table alias", () => {
    const groups = columnGroups("select id, name from driver d", [
      { name: "id", typeName: "INT" },
      { name: "name", typeName: "VARCHAR" },
    ]);
    expect(groups).toEqual([
      {
        alias: "d",
        columns: [
          { index: 0, name: "id", typeName: "INT" },
          { index: 1, name: "name", typeName: "VARCHAR" },
        ],
      },
    ]);
  });

  it("splits a star across aliases when the schema is known", () => {
    const groups = columnGroups("select d.*, t.started_at from fleet.driver d join trip t on t.id = d.id", [
      { name: "id", typeName: "INT" },
      { name: "name", typeName: "VARCHAR" },
      { name: "started_at", typeName: "DATETIME" },
    ], driverTrip, "fleet");
    expect(groups.map((group) => [group.alias, group.columns.map((column) => column.name)])).toEqual([
      ["d", ["id", "name"]],
      ["t", ["started_at"]],
    ]);
  });

  it("assigns a bare star on one table to that alias", () => {
    const groups = columnGroups("select * from driver", [
      { name: "id", typeName: "INT" },
      { name: "name", typeName: "VARCHAR" },
    ]);
    expect(groups.map((group) => group.alias)).toEqual(["driver"]);
  });
});
