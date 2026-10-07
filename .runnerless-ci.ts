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
    const names = ["starlink-core", "starlink-proto", "starlink-cli"];
    for (let i = 0; i < names.length; i++) {
      const result = jobs.lookup<WriteResult>(new OperationRef<WriteResult>("op:publish-" + names[i]));
      if (result.ok) {
        const value = result.value;
        checks.report("crates-" + names[i], value.get("status").string() == "succeeded"
          && value.get("value").get("failed").json == "false",
          "crates.io publication: " + value.get("status").string() + " " + value.get("value").get("error").string());
      }
    }
    return;
  }
  if (details.get("event").string() != "workflow_run" || details.get("action").string() != "completed"
    || details.get("workflowPath").string() != ".github/workflows/publish-release.yml"
    || details.get("conclusion").string() != "success") return;
  const version = repo.readText("version.txt", "head").trim();
  const tag = "v" + version;
  if (!packages.releasePublished(tag, version)) return;
  const core = packages.crates(new OperationRequest("publish-starlink-core"),
    new RegistryPublishRequest(tag, version).add("starlink-core", "starlink-core-" + version + ".cargo-upload"));
  const proto = packages.crates(new OperationRequest("publish-starlink-proto").after(core),
    new RegistryPublishRequest(tag, version).add("starlink-proto", "starlink-proto-" + version + ".cargo-upload"));
  packages.crates(new OperationRequest("publish-starlink-cli").after(proto),
    new RegistryPublishRequest(tag, version).add("starlink-cli", "starlink-cli-" + version + ".cargo-upload"));
}
