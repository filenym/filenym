import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { cleanup } from "@testing-library/react";
import AppShell from "./AppShell";

afterEach(cleanup);

describe("AppShell", () => {
  it("renders the FileNym application shell", () => {
    render(<AppShell version="0.1.0" />);

    expect(screen.getByRole("heading", { name: "FileNym" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "FileNym logo" })).toHaveAttribute(
      "src",
      "/filenym-logo.png",
    );
    expect(screen.getByText("One file. Many names.")).toBeInTheDocument();
    expect(screen.getByText("v0.1.0")).toBeInTheDocument();
  });
});
