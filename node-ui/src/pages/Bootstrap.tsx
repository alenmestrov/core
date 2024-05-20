import React from "react";
import { setupWalletSelector } from "@near-wallet-selector/core";
import { setupMyNearWallet } from "@near-wallet-selector/my-near-wallet";
import { Buffer } from "buffer";
import axios from "axios";
import { Login } from "../components/login/Login";
import { Footer } from "../components/footer/Footer";
import MetamaskContext from "@calimero-is-near/calimero-p2p-sdk/lib/wallet/MetamaskLogin/MetamaskWrapper";

import styled from "styled-components";
import { getWalletCallbackUrl } from "../utils/wallet";


export interface Challenge {
  nonce: string;
  applicationId: string;
  timestamp: number;
  nodeSignature: string;
}

const fetchChallenge = async (): Promise<Challenge> => {
  const body = {
    applicationId: "admin-ui",
  };
  const response = await axios.post("/admin-api/request-challenge", body);
  const payload: Challenge = response.data.data;
  return payload;
};

const verifyOwner = async (): Promise<void> => {
  let challengeObject: null | Challenge = null;
  try {
    challengeObject = await fetchChallenge();
    console.log("🚀 ~ verifyOwner ~ challengeObject:", challengeObject)
  } catch (e) {
    console.error("Failed to fetch challenge:", e);
    return;
  }
  const nonce = Buffer.from(challengeObject.nonce, "base64");
  const selector = await setupWalletSelector({
    network: "testnet",
    modules: [setupMyNearWallet()],
  });
  const wallet = await selector.wallet("my-near-wallet");
  const callbackUrl = getWalletCallbackUrl();
  const message = challengeObject.nodeSignature;
  const recipient = "me";
  console.log("Signing messagex:", {
    message,
    recipient,
    nonceBase64: nonce,
    callbackUrl,
  });
  await wallet.signMessage({ message, nonce, recipient, callbackUrl });
};

const BootstrapWrapper = styled.div`
  height: 150px;
`;

function Bootstrap(): JSX.Element {
  console.log("da ti jebem mater");
  return (
    <BootstrapWrapper>
       <MetamaskContext
        applicationId={"node-ui"}
        rpcBaseUrl={"http://localhost:2428"}
        successRedirect={() => console.log("sucess")}
        navigateBack={() => console.log("nav back")}
        addRootKey={true}
      />
    </BootstrapWrapper>
  );
}

export default Bootstrap;
