import React, { useState } from "react";
import {
  MetaMaskButton,
  useAccount,
  useSDK,
  // @ts-ignore: sdk-react-ui does not export useSignMessage
  useSignMessage,
} from "@metamask/sdk-react-ui";
import apiClient from "../../api";
import { NodeChallenge } from "../../nodeApi";
import { ResponseData } from "../../api-response";
import { Loading } from "../loading/Loading";

interface LoginWithMetamaskProps {
  applicationId: string;
  rpcBaseUrl: string;
  successRedirect: () => void;
  metamaskTitleColor: string | undefined;
  navigateBack: () => void | undefined;
  addRootKey: boolean;
}

export default function LoginWithMetamask({
  applicationId,
  rpcBaseUrl,
  successRedirect,
  metamaskTitleColor,
  navigateBack,
  addRootKey,
}: LoginWithMetamaskProps) {
  const { isConnected, address } = useAccount();
  const { ready } = useSDK();
  const [signedMessage, setMessage] = useState("default");

  const { data: signData, signMessage } = useSignMessage({
    message: signedMessage,
  });

  const requestChallenge = async () => {
    console.log(applicationId);
    console.log(addRootKey);
    const challengeResponseData: ResponseData<NodeChallenge> = await apiClient
      .node()
      .requestChallenge(rpcBaseUrl, "admin-ui");

    const message = challengeResponseData.data.nodeSignature;

    setMessage(JSON.stringify(message));
  };
  const signMessageOnClick = async () => {
    signMessage();
  };

  const confirm = async () => {
    console.log(signData);

    const rootKeyParams = {
      accountId: address,
      signature: signData,
      publicKey: address,
      callbackUrl: "http://localhost:3000",
    };
    console.log("🚀 ~ confirm ~ rootKeyParams:", rootKeyParams);
    await apiClient
      .node()
      .addRootKey(rootKeyParams, rpcBaseUrl)
      .then((result) => {
        console.log(result);
      })
      .catch(() => {
        console.error("error while login");
      });
  };

  if (!ready) {
    return <Loading />;
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        padding: "0.5rem",
      }}
    >
      <div
        style={{
          marginTop: "1.5rem",
          display: "grid",
          color: "white",
          fontSize: "1.25rem",
          fontWeight: "500",
          textAlign: "center",
        }}
      >
        <span
          style={{
            marginBottom: "0.5rem",
            color: metamaskTitleColor ?? "#fff",
          }}
        >
          Metamask
        </span>
        <header
          style={{
            marginTop: "1.5rem",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <MetaMaskButton
            theme="dark"
            color={isConnected ? "blue" : "white"}
            buttonStyle={
              isConnected
                ? {
                    display: "flex",
                    justifyContent: "center",
                    alignItems: "center",
                    backgroundColor: "#25282D",
                    height: "73px",
                    borderRadius: "6px",
                    border: "none",
                    outline: "none",
                  }
                : {
                    cursor: "pointer",
                  }
            }
          ></MetaMaskButton>
          {isConnected && (
            <>
              <div style={{ marginTop: "155px" }}>
                <button
                  style={{
                    backgroundColor: "#FF7A00",
                    color: "white",
                    width: "100%",
                    display: "flex",
                    justifyContent: "center",
                    alignItems: "center",
                    gap: "0.5rem",
                    height: "46px",
                    cursor: "pointer",
                    fontSize: "1rem",
                    fontWeight: "500",
                    borderRadius: "0.375rem",
                    border: "none",
                    outline: "none",
                    paddingLeft: "0.5rem",
                    paddingRight: "0.5rem",
                  }}
                  onClick={() => signMessageOnClick()}
                >
                  Sign authentication transaction
                </button>
              </div>
              <div style={{ marginTop: "155px" }}>
                <button
                  style={{
                    backgroundColor: "#FF7A00",
                    color: "white",
                    width: "100%",
                    display: "flex",
                    justifyContent: "center",
                    alignItems: "center",
                    gap: "0.5rem",
                    height: "46px",
                    cursor: "pointer",
                    fontSize: "1rem",
                    fontWeight: "500",
                    borderRadius: "0.375rem",
                    border: "none",
                    outline: "none",
                    paddingLeft: "0.5rem",
                    paddingRight: "0.5rem",
                  }}
                  onClick={() => requestChallenge()}
                >
                  RequestChallenge
                </button>
                <button
                  style={{
                    backgroundColor: "#FF7A00",
                    color: "white",
                    width: "100%",
                    display: "flex",
                    justifyContent: "center",
                    alignItems: "center",
                    gap: "0.5rem",
                    height: "46px",
                    cursor: "pointer",
                    fontSize: "1rem",
                    fontWeight: "500",
                    borderRadius: "0.375rem",
                    border: "none",
                    outline: "none",
                    paddingLeft: "0.5rem",
                    paddingRight: "0.5rem",
                  }}
                  onClick={() => confirm()}
                >
                  confirm
                </button>
              </div>
            </>
          )}
        </header>
      </div>
      <div
        style={{
          paddingTop: "1rem",
          fontSize: "14px",
          color: "#fff",
          textAlign: "center",
          cursor: "pointer",
        }}
        onClick={navigateBack}
      >
        Back to wallet selector
      </div>
    </div>
  );
}
