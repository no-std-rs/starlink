import {
  workflow, configuration, event, repo, packages, checks, jobs,
  OperationRef, OperationRequest, RegistryPublishRequest, WriteResult,
} from "runnerless/v1";

const program = workflow().releasePlease();

export function configure(): void {
  program.configure();
  configuration.workflow("crate-publishing", ["workflow_run", "runnerless_completion"]);
}

export function run(): void {
  program.run();
  const details = event.details();
  if (details.get("event").string() == "runnerless_completion") {
    const result = jobs.lookup<WriteResult>(new OperationRef<WriteResult>("op:crates-publish"));
    if (result.ok) {
      const value = result.value;
      checks.report("crates-publish", value.get("status").string() == "succeeded"
        && value.get("value").get("failed").json == "false",
        "crates.io publication: " + value.get("status").string() + " " + value.get("value").get("error").string());
    }
    return;
  }
  if (details.get("event").string() != "workflow_run" || details.get("action").string() != "completed"
    || details.get("workflowPath").string() != ".github/workflows/publish-release.yml"
    || details.get("conclusion").string() != "success") return;
  const version = repo.readText("version.txt", "head").trim();
  const tag = "v" + version;
  if (!packages.releasePublished(tag, version)) return;
  const request = new RegistryPublishRequest(tag, version);
  request.add("starlink-core", "starlink-core-" + version + ".cargo-upload");
  request.add("starlink-proto", "starlink-proto-" + version + ".cargo-upload");
  request.add("starlink-cli", "starlink-cli-" + version + ".cargo-upload");
  packages.crates(new OperationRequest("crates-publish"), request);
}
