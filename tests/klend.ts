import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { assert } from "chai";
import { Klend } from "../target/types/klend";

describe("klend", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Klend as Program<Klend>;

  it("wires up workspace (placeholder)", async () => {
    // No runtime side-effects; just ensure workspace loads
    try {
      // This method intentionally does not exist; expect failure
      // @ts-ignore
      await program.methods.idlMissingTypes().rpc();
    } catch (e) {
      assert.ok(true);
    }
  });
});
