import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Card } from "./Card";

function classes(element: Element): string[] {
  return [...element.children].map((child) => child.className.split(" ")[0]);
}

describe("Card", () => {
  it("without a side column is the title row and the body, as before", () => {
    const { container } = render(<Card title="T">body</Card>);
    const card = container.firstElementChild as HTMLElement;
    expect(card.className).toBe("card");
    expect(classes(card)).toEqual(["card__title-row", "card__body"]);
  });

  it("keeps the footer after the body without a side column", () => {
    const { container } = render(
      <Card title="T" footer={<b>f</b>}>
        body
      </Card>,
    );
    expect(classes(container.firstElementChild as Element)).toEqual(["card__title-row", "card__body", "card__footer"]);
  });

  it("with a side column puts the title, body and footer in a main part and the side column beside it, full height", () => {
    const { container } = render(
      <Card title="T" footer={<b>f</b>} side={<i>side</i>}>
        body
      </Card>,
    );
    const card = container.firstElementChild as HTMLElement;
    expect(card.className).toBe("card card--with-side");
    expect(classes(card)).toEqual(["card__main", "card__side"]);
    expect(classes(card.querySelector(".card__main") as Element)).toEqual([
      "card__title-row",
      "card__body",
      "card__footer",
    ]);
    expect(card.querySelector(".card__side")?.textContent).toBe("side");
  });

  it("keeps a custom class next to card--with-side", () => {
    const { container } = render(
      <Card title="T" className="tempo-card" side={<i>s</i>}>
        b
      </Card>,
    );
    expect((container.firstElementChild as HTMLElement).className).toBe("card tempo-card card--with-side");
  });
});
