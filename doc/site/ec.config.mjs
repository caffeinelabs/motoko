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

export default defineEcConfig({
  shiki: {
    langs: [motoko, candid],
    langAlias: {
      mo: "motoko",
      did: "candid",
    },
  },
});
