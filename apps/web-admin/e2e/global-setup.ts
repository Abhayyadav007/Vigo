import { backendCli } from "./helpers";

/** Demo stores give the Staff test a store to assign riders to. */
export default function globalSetup() {
  backendCli("seed-demo");
}
