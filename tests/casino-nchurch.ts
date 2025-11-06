import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { CasinoNchurch } from "../target/types/casino_nchurch";
import { PublicKey, Keypair, SystemProgram, Commitment } from "@solana/web3.js";
import { createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID, mintTo, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import fs from "fs";
import * as sb from "@switchboard-xyz/on-demand";
import path from "path";

describe("casino-nchurch", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const signerKeypair = Keypair.generate();

  let CNC_MINT: PublicKey = new PublicKey("83czKg4PVLbmUcWhjpq5AXnSivDJrJVMf3AdwuUCPhPT");
  let USDC_MINT_DECIMALS = 6;

  const secretKeyString = fs.readFileSync("/Users/tsmboa/.config/solana/id.json", "utf8");
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));

  const programKeypair = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync("/Users/tsmboa/dev/casino-nchurch/target/deploy/casino_nchurch-keypair.json", "utf8"))));
  console.log("programKeypair", programKeypair.publicKey.toBase58());

  const signer = Keypair.fromSecretKey(secretKey);
  const connection_ = new anchor.web3.Connection("http://127.0.0.1:8899", "confirmed");
  const connection2 = new anchor.web3.Connection("https://devnet.helius-rpc.com/?api-key=193b6782-795f-4f9f-a39c-838bfc136663", "confirmed");
  const wallet = new anchor.Wallet(signer);
  const provider = new anchor.AnchorProvider(connection_, wallet, { commitment: "confirmed" });
  const provider2 = new anchor.AnchorProvider(connection2, wallet, { commitment: "confirmed" });
  

  const idl = JSON.parse(fs.readFileSync("target/idl/casino_nchurch.json", "utf8"));
  // const program_ = new anchor.Program(idl, provider);

  const program_ = anchor.workspace.CasinoNchurch as Program<CasinoNchurch>;

  console.log("program_", program_.programId.toBase58());
  
  anchor.setProvider(provider);

  const RANDOMNESS_KEYPAIR_PATH = path.join(process.cwd(), "tests", "randomness-keypair.json");


  //============== switchboard setup ==============
  const loadSbProgram = async (provider: anchor.Provider) => {
    const sbProgramId = await sb.getProgramId(provider.connection);
    const sbIdl = await anchor.Program.fetchIdl(sbProgramId, provider);
    return new anchor.Program(sbIdl!, provider);
  };

  const setupQueue = async (program: anchor.Program): Promise<PublicKey> => {
    const queueAccount = await sb.getDefaultQueue(
      program.provider.connection.rpcEndpoint
    );
    console.log("Queue account", queueAccount.pubkey.toString());
    try {
      await queueAccount.loadData();
    } catch (err) {
      console.error("Queue not found, ensure you are using devnet in your env");
      process.exit(1);
    }
    return queueAccount.pubkey;
  }

  const loadOrCreateRandomnessAccount = async (sbProgram: anchor.Program, queue: any): Promise<{ randomness: sb.Randomness; rngKp: Keypair; createIx?: anchor.web3.TransactionInstruction }> => {
    let rngKp: Keypair;
    let createIx: anchor.web3.TransactionInstruction | undefined;
  
    if (fs.existsSync(RANDOMNESS_KEYPAIR_PATH)) {
      console.log("Loading existing randomness account...");
      const keypairData = JSON.parse(fs.readFileSync(RANDOMNESS_KEYPAIR_PATH, 'utf8'));
      rngKp = Keypair.fromSecretKey(new Uint8Array(keypairData));
      console.log("Loaded randomness account", rngKp.publicKey.toString());
  
      const randomness = new sb.Randomness(sbProgram, rngKp.publicKey);
      return { randomness, rngKp };
    } else {
      console.log("Creating new randomness account...");
      rngKp = Keypair.generate();
  
      fs.writeFileSync(RANDOMNESS_KEYPAIR_PATH, JSON.stringify(Array.from(rngKp.secretKey)));
      console.log("Saved randomness keypair to", RANDOMNESS_KEYPAIR_PATH);
      console.log("randomness keypair", rngKp.publicKey.toString());
  
      const [randomness, ix] = await sb.Randomness.create(sbProgram, rngKp, queue);
      console.log("Created randomness account", randomness.pubkey.toString());
  
      return { randomness, rngKp, createIx: ix };
    }
  }

  async function retryCommitRandomness(randomness: sb.Randomness, queue: any, maxRetries: number = 3, delayMs: number = 2000): Promise<anchor.web3.TransactionInstruction> {
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        console.log(`Attempting to commit randomness (attempt ${attempt}/${maxRetries})...`);
        const commitIx = await randomness.commitIx(queue);
        console.log("Successfully obtained commit instruction");
        return commitIx;
      } catch (error: any) {
        console.log(`Commit attempt ${attempt} failed:`, error.message);
  
        if (attempt === maxRetries) {
          console.error("All commit attempts failed. The Switchboard gateway may be experiencing issues.");
          throw error;
        }
  
        console.log(`Waiting ${delayMs}ms before retry...`);
        await new Promise(resolve => setTimeout(resolve, delayMs));
  
        // Increase delay for next attempt
        delayMs = Math.min(delayMs * 1.5, 8000);
      }
    }
  
    throw new Error("Should not reach here");
  }

  async function retryRevealRandomness(randomness: sb.Randomness, maxRetries: number = 5, delayMs: number = 2000): Promise<anchor.web3.TransactionInstruction> {
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        console.log(`Attempting to reveal randomness (attempt ${attempt}/${maxRetries})...`);
        const revealIx = await randomness.revealIx();
        console.log("Successfully obtained reveal instruction");
        return revealIx;
      } catch (error: any) {
        console.log(`Reveal attempt ${attempt} failed:`, error.message);
  
        if (attempt === maxRetries) {
          console.error("All reveal attempts failed. The Switchboard gateway may be experiencing issues.");
          throw error;
        }
  
        console.log(`Waiting ${delayMs}ms before retry...`);
        await new Promise(resolve => setTimeout(resolve, delayMs));
  
        // Increase delay for next attempt (exponential backoff)
        delayMs = Math.min(delayMs * 1.5, 10000);
      }
    }
  
    throw new Error("Should not reach here");
  }

  before(async () => {
    console.log("creating test mint and transferin some sol and usdc to the casino vault")
    // const [casinoStatePDA, bump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("casino_state")], program_.programId);
    // Create a test mint (your own "mock USDC")
    const usdcMint = await createMint(
      connection2,
      signer,
      signer.publicKey,
      null,
      6 // decimals
    );

    const userAddress = new PublicKey("HgNrCxgsuNdepRRbDsqQEYvVAnKRQS9HW7bbBMHcce8G");

    console.log("Test mint created", usdcMint.toBase58());
    CNC_MINT = usdcMint;
    // Get or create your token account
    try{
      console.log("getting or creating user token account");
      const userATA = await getOrCreateAssociatedTokenAccount(
        connection2,
        signer,
        CNC_MINT,
        userAddress
      );

      console.log("user token account created", userATA.address.toBase58());

      // Mint 1000 USDC (1_000 * 10^6)
      await mintTo(
        connection2,
        signer,
        CNC_MINT,
        userATA.address,
        signer.publicKey,
        1_000_000_000_000 // 1000 * 10^6
      );

      // fund vault with sol and usdc

      console.log("🪙 Minted 1000 test USDC to", userATA.address.toBase58());
    }catch(error){
      console.error("Error getting or creating user token account", error);
    }

  });


  it("Initialize casino!", async () => {
    // Add your test here.
    const [casinoStatePDA, bump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("casino_state")], program_.programId);
    const casinoVaultPDA = getAssociatedTokenAddressSync(CNC_MINT, casinoStatePDA, true);

    console.log("Proceeding to initialize the casino");

    const initTx = await program_.methods.initializeCasino().accountsStrict({
      casinoState: casinoStatePDA,
      casinoVault: casinoVaultPDA,
      authority: signer.publicKey,
      usdcMint: CNC_MINT,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    }).signers([signer]).rpc();

    console.log("Initialize casino transaction signature", initTx);

    // Get or create the casino vault token account
    const casinoVaultATA = await getOrCreateAssociatedTokenAccount(
      connection_,
      signer,
      CNC_MINT,
      casinoStatePDA,
      true,
    );

    await mintTo(
      connection_,
      signer,
      CNC_MINT,
      casinoVaultATA.address,
      signer.publicKey,
      1_000_000_000_000 // 1000 * 10^6
    );

    console.log("Funded casino vault with 1000 USDC");
    console.log("casinoVaultATA", casinoVaultATA.address.toBase58());
  });

  // it("Request slots game!", async () => {
  //   console.log("Proceeding to request slot game!!")
  //   const [casinoStatePDA, bump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("casino_state")], program_.programId);
  //   const casinoVaultPDA = getAssociatedTokenAddressSync(USDC_MINT, casinoStatePDA, true);

  //   const [vrfGameStatePDA, vrfGameStateBump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("vrf_game_state"), signer.publicKey.toBuffer()], program_.programId);
  //   const [userStatsPDA, userStatsBump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("user_stats"), signer.publicKey.toBuffer()], program_.programId);
  //   const userTokenAccountPDA = getAssociatedTokenAddressSync(USDC_MINT, signer.publicKey);

  //   console.log("casinoStatePDA", casinoStatePDA.toBase58());
  //   console.log("casinoVaultPDA", casinoVaultPDA.toBase58());
  //   console.log("vrfGameStatePDA", vrfGameStatePDA.toBase58());
  //   console.log("userStatsPDA", userStatsPDA.toBase58());
  //   console.log("userTokenAccountPDA", userTokenAccountPDA.toBase58());

  //   const { keypair, connection, program } = await sb.AnchorUtils.loadEnv();
  //   console.log("programId for the environment is", program.programId.toBase58());
  //   console.log("sb anchor utils loaded");

  //   // load or create randomness account
  //   const sbProgram = await loadSbProgram(program!.provider);
  //   console.log("sb program loaded", sbProgram.programId.toBase58());
  //   const queue = await setupQueue(program!);
  //   const { randomness, rngKp, createIx } = await loadOrCreateRandomnessAccount(sbProgram, queue);

  //   const txOpts = {
  //     commitment: "processed" as Commitment,
  //     skipPreflight: false,
  //     maxRetries: 0,
  //   };

  //   // Only create the randomness account if it's new
  //   if (createIx) {
  //     const createRandomnessTx = await sb.asV0Tx({
  //       connection: connection2,
  //       ixs: [createIx],
  //       payer: signer.publicKey,
  //       signers: [signer, rngKp],
  //       computeUnitPrice: 75_000,
  //       computeUnitLimitMultiple: 1.3,
  //     });

  //     console.log("attempting to send the create randomness transaction.")

  //     const sim = await connection.simulateTransaction(createRandomnessTx, txOpts);
  //     const sig1 = await connection.sendTransaction(createRandomnessTx, txOpts);
  //     await connection.confirmTransaction(sig1, "processed");
  //     console.log(
  //       "  Transaction Signature for randomness account creation: ",
  //       sig1
  //     );
  //   } else {
  //     console.log("Reusing existing randomness account:", randomness.pubkey.toString());
  //   }

  //   //create randomness commit instruction
  //   const commitIx = await retryCommitRandomness(randomness, queue);
  //   console.log("Randomness commit instruction", commitIx);

  //   const betAmount = 1000000;
  //   const randomnessAccount = rngKp.publicKey;

  //   const requestSlotsGameTx = await program_.methods.requestSlotsGame(new BN(betAmount), randomnessAccount).accountsStrict({
  //     casinoState: casinoStatePDA,
  //     casinoVault: casinoVaultPDA,
  //     vrfGameState: vrfGameStatePDA,
  //     userStats: userStatsPDA,
  //     user: signer.publicKey,
  //     userTokenAccount: userTokenAccountPDA,
  //     usdcMint: USDC_MINT,
  //     randomnessAccountData: rngKp.publicKey,
  //     systemProgram: SystemProgram.programId,
  //     tokenProgram: TOKEN_PROGRAM_ID,
  //     associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //   }).signers([signer]).instruction();

  //   console.log("Request slots game transaction signature", requestSlotsGameTx);

  //   const tx = await sb.asV0Tx({
  //     connection: connection2,
  //     ixs: [commitIx],
  //     payer: signer.publicKey,
  //     signers: [signer],
  //     computeUnitPrice: 75_000,
  //     computeUnitLimitMultiple: 1.3,
  //   });
  //   console.log("Transaction for commitRNG", tx);

  //   const sig = await connection.sendTransaction(tx, txOpts);
  //   await connection_.confirmTransaction(sig, "processed");
  //   console.log("Transaction signature for commitRNG", sig);

  //   console.log("sending the request slot game transaction.")

  //   const txG = await sb.asV0Tx({
  //     connection: connection_,
  //     ixs:[requestSlotsGameTx],
  //     payer:signer.publicKey,
  //     signers:[signer]
  //   })

  //   const sigG = await connection_.sendTransaction(txG, txOpts)
  //   console.log("Trasaction sent successfully! Slot game created.")

  //   // const vrfGameState = await program_.account.vrfGameState.fetch(vrfGameStatePDA);
  //   // console.log("VRF game state", vrfGameState);
  // });

  // it("Settle slots game!", async () => {
  //   const [casinoStatePDA, bump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("casino_state")], program_.programId);
  //   const casinoVaultPDA = getAssociatedTokenAddressSync(USDC_MINT, casinoStatePDA, true);

  //   const [vrfGameStatePDA, vrfGameStateBump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("vrf_game_state"), signer.publicKey.toBuffer()], program_.programId);
  //   const [userStatsPDA, userStatsBump] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("user_stats"), signer.publicKey.toBuffer()], program_.programId);
  //   const userTokenAccountPDA = getAssociatedTokenAddressSync(USDC_MINT, signer.publicKey);

  //   const { keypair, connection, program } = await sb.AnchorUtils.loadEnv();
  //   console.log("sb anchor utils loaded");

  //   // load or create randomness account
  //   const sbProgram = await loadSbProgram();
  //   const queue = await setupQueue(program);
  //   const { randomness, rngKp, createIx } = await loadOrCreateRandomnessAccount(sbProgram, queue);

  //   const txOpts = {
  //     commitment: "processed" as Commitment,
  //     skipPreflight: false,
  //     maxRetries: 0,
  //   };

  //   //wait for 2 seconds
  //   await new Promise(resolve => setTimeout(resolve, 2000));

  //   //create randomness reveal instruction
  //   const revealIx = await retryRevealRandomness(randomness);
  //   console.log("Randomness reveal instruction", revealIx);

  //   const settleSlotsGameTx = await program_.methods.settleSlotsGame().accountsStrict({
  //     casinoState: casinoStatePDA,
  //     casinoVault: casinoVaultPDA,
  //     vrfGameState: vrfGameStatePDA,
  //     userStats: userStatsPDA,
  //     user: signer.publicKey,
  //     userTokenAccount: userTokenAccountPDA,
  //     randomnessAccountData: rngKp.publicKey,
  //     usdcMint: USDC_MINT,
  //     tokenProgram: TOKEN_PROGRAM_ID,
  //     systemProgram: SystemProgram.programId,
  //     associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //   }).signers([signer]).instruction();

  //   console.log("Settle slots game transaction signature", settleSlotsGameTx);

  //   const tx = await sb.asV0Tx({
  //     connection: connection_,
  //     ixs: [revealIx, settleSlotsGameTx],
  //     payer: keypair.publicKey,
  //     signers: [keypair, rngKp],
  //     computeUnitPrice: 75_000,
  //     computeUnitLimitMultiple: 1.3,
  //   });
  //   console.log("Transaction", tx);

  //   const sig = await connection_.sendTransaction(tx, txOpts);
  //   await connection_.confirmTransaction(sig, "processed");
  //   console.log("Transaction signature", sig);

  //   // const vrfGameState = await program_.account.vrfGameState.fetch(vrfGameStatePDA);
  //   // console.log("VRF game state", vrfGameState);

  // });

});
