import { workflow } from "runnerless/v1";

const program = workflow().releasePlease();

export function configure(): void { program.configure(); }
export function run(): void { program.run(); }
