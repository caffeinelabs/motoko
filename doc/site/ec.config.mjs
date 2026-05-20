import { defineEcConfig } from "@astrojs/starlight/expressive-code";
import fs from "node:fs";

const motoko = {
  ...JSON.parse(fs.readFileSync("./syntaxes/motoko.tmLanguage.json", "utf-8")),
  name: "motoko",
};
const candid = {
  ...JSON.parse(fs.readFileSync("./syntaxes/candid.tmLanguage.json", "utf-8")),
  name: "candid",
};
const bnf = {
  ...JSON.parse(fs.readFileSync("./syntaxes/bnf.tmLanguage.json", "utf-8")),
  name: "bnf",
};

export default defineEcConfig({
  shiki: {
    langs: [motoko, candid, bnf],
    langAlias: {
      mo: "motoko",
      did: "candid",
    },
  },
});
