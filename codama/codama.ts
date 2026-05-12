import { renderVisitor as renderJavaScriptVisitor } from "@codama/renderers-js";
import { renderVisitor as renderRustVisitor } from "@codama/renderers-rust";
import {
  arrayTypeNode,
  constantPdaSeedNodeFromString,
  createFromRoot,
  definedTypeLinkNode,
  definedTypeNode,
  errorNode,
  instructionAccountNode,
  instructionArgumentNode,
  instructionNode,
  numberTypeNode,
  numberValueNode,
  pdaNode,
  prefixedCountNode,
  programNode,
  publicKeyTypeNode,
  publicKeyValueNode,
  rootNode,
  structFieldTypeNode,
  structTypeNode,
  variablePdaSeedNode,
} from "codama";
import path from "path";
import fs from "fs";

// === DIRECTORY CONFIGURATION ===
const typescriptClientsDir = path.join(__dirname, "..", "sdk", "ts");
const rustClientsDir = path.join(__dirname, "..", "sdk", "rust");

// === PROGRAM DEFINITION ===
const root = rootNode(
  programNode({
    name: "nonceGuard",
    publicKey: "ExtSgUPtP3JyKUysFw2S5fpL5fWfUPzGUQLd2bTwftXN",
    version: "0.1.0",
    accounts: [],
    pdas: [
      pdaNode({
        name: "guardedAccount",
        seeds: [
          constantPdaSeedNodeFromString("utf8", "guarded_proxy"),
          variablePdaSeedNode("owner", publicKeyTypeNode()),
        ],
      }),
    ],
    definedTypes: [
      definedTypeNode({
        name: "compiledInstruction",
        type: structTypeNode([
          structFieldTypeNode({
            name: "programIdIndex",
            type: numberTypeNode("u8"),
            docs: [
              "Index into the remaining accounts array for the target program. Index 0 is reserved for the guarded PDA.",
            ],
          }),
          structFieldTypeNode({
            name: "accountIndices",
            type: arrayTypeNode(
              numberTypeNode("u8"),
              prefixedCountNode(numberTypeNode("u32"))
            ),
            docs: [
              "Indices into the remaining accounts array for each account the CPI needs. Index 0 is the guarded PDA.",
            ],
          }),
          structFieldTypeNode({
            name: "data",
            type: arrayTypeNode(
              numberTypeNode("u8"),
              prefixedCountNode(numberTypeNode("u32"))
            ),
            docs: ["Raw instruction data for the CPI."],
          }),
        ]),
      }),
    ],
    instructions: [
      instructionNode({
        name: "executeGuarded",
        docs: [
          "Executes one or more CPIs through the guarded PDA proxy.",
          "The guarded PDA signs every proxied CPI. Rejects transactions that use a durable nonce.",
        ],
        discriminators: [],
        arguments: [
          instructionArgumentNode({
            name: "instructions",
            type: arrayTypeNode(
              definedTypeLinkNode("compiledInstruction"),
              prefixedCountNode(numberTypeNode("u8"))
            ),
            docs: [
              "List of compiled instructions to execute via CPI. Uses a u8 length prefix.",
            ],
          }),
        ],
        accounts: [
          instructionAccountNode({
            name: "owner",
            isSigner: true,
            isWritable: false,
            docs: ["Owner of the guarded PDA. Must be a signer."],
          }),
          instructionAccountNode({
            name: "guardedAccount",
            isSigner: false,
            isWritable: true,
            docs: [
              "Guarded PDA derived from the owner. Signs all proxied CPIs.",
            ],
          }),
          instructionAccountNode({
            name: "instructionsSysvar",
            defaultValue: publicKeyValueNode(
              "Sysvar1nstructions1111111111111111111111111",
              "instructionsSysvar"
            ),
            isSigner: false,
            isWritable: false,
            docs: [
              "Instructions sysvar, used to check for durable nonce usage.",
            ],
          }),
        ],
        optionalAccountStrategy: "omitted",
      }),
    ],
    errors: [
      errorNode({
        code: 0,
        name: "invalidInstructionData",
        message: "Invalid instruction data.",
      }),
      errorNode({
        code: 1,
        name: "ownerMustSign",
        message: "Owner must sign the transaction.",
      }),
      errorNode({
        code: 2,
        name: "invalidGuardedPda",
        message: "Invalid guarded PDA address.",
      }),
      errorNode({
        code: 3,
        name: "accountIndexOutOfBounds",
        message: "Account index out of bounds.",
      }),
      errorNode({
        code: 4,
        name: "programIndexOutOfBounds",
        message: "Program index out of bounds.",
      }),
      errorNode({
        code: 5,
        name: "selfCpiBlocked",
        message: "Self-CPI is not allowed.",
      }),
      errorNode({
        code: 6,
        name: "durableNonceBlocked",
        message: "Transactions using durable nonces are blocked.",
      }),
    ],
  })
);

// === PRESERVE CONFIG FILES ===
function preserveConfigFiles() {
  const tsFilesToPreserve = [
    "package.json",
    "tsconfig.json",
    ".npmignore",
    "pnpm-lock.yaml",
  ];
  const rustFilesToPreserve = ["Cargo.toml", "Cargo.lock"];
  const preservedFiles = new Map<string, string>();

  tsFilesToPreserve.forEach((filename) => {
    const filePath = path.join(typescriptClientsDir, filename);
    const tempPath = path.join(typescriptClientsDir, `${filename}.temp`);
    if (fs.existsSync(filePath)) {
      fs.copyFileSync(filePath, tempPath);
      preservedFiles.set(filePath, tempPath);
    }
  });

  rustFilesToPreserve.forEach((filename) => {
    const filePath = path.join(rustClientsDir, filename);
    const tempPath = path.join(rustClientsDir, `${filename}.temp`);
    if (fs.existsSync(filePath)) {
      fs.copyFileSync(filePath, tempPath);
      preservedFiles.set(filePath, tempPath);
    }
  });

  const libRsPath = path.join(rustClientsDir, "src", "lib.rs");
  const libRsTempPath = path.join(rustClientsDir, "src", "lib.rs.temp");
  if (fs.existsSync(libRsPath)) {
    fs.copyFileSync(libRsPath, libRsTempPath);
    preservedFiles.set(libRsPath, libRsTempPath);
  }

  return {
    restore: () => {
      preservedFiles.forEach((tempPath, originalPath) => {
        if (fs.existsSync(tempPath)) {
          fs.copyFileSync(tempPath, originalPath);
          fs.unlinkSync(tempPath);
        }
      });
    },
  };
}

// === GENERATE SDKs ===
const codama = createFromRoot(root);
const configPreserver = preserveConfigFiles();

codama.accept(
  renderJavaScriptVisitor(typescriptClientsDir, {
    formatCode: true,
    deleteFolderBeforeRendering: true,
  })
);

codama.accept(
  renderRustVisitor(rustClientsDir, {
    crateFolder: rustClientsDir,
    formatCode: true,
    deleteFolderBeforeRendering: true,
  })
);

configPreserver.restore();

console.log("SDK generation complete!");
console.log(`  TypeScript: ${typescriptClientsDir}/src/`);
console.log(`  Rust: ${rustClientsDir}/src/client/`);
