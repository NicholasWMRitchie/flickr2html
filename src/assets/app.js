(function () {
  "use strict";
  const tiles = Array.from(document.querySelectorAll(".tile"));
  if (tiles.length === 0) return;

  const lb = document.getElementById("lightbox");
  if (!lb) return;
  const media = lb.querySelector(".lb-media");
  const elName = lb.querySelector(".lb-name");
  const elDesc = lb.querySelector(".lb-desc");
  const elDate = lb.querySelector(".lb-date");
  const elExif = lb.querySelector(".lb-exif");
  const btnClose = lb.querySelector(".lb-close");
  const btnPrev = lb.querySelector(".lb-prev");
  const btnNext = lb.querySelector(".lb-next");

  let current = -1;

  function show(i) {
    if (i < 0 || i >= tiles.length) return;
    current = i;
    const t = tiles[i];
    const kind = t.dataset.kind || "image";
    const full = t.dataset.full || "";
    const name = t.dataset.name || "";
    const desc = t.dataset.desc || "";
    const date = t.dataset.date || "";
    const exif = t.dataset.exif || "";

    media.innerHTML = "";
    if (kind === "video") {
      const v = document.createElement("video");
      v.src = full;
      v.controls = true;
      v.preload = "metadata";
      v.playsInline = true;
      media.appendChild(v);
    } else {
      const img = document.createElement("img");
      img.src = full;
      img.alt = name;
      media.appendChild(img);
    }
    elName.textContent = name;
    elDesc.textContent = desc;
    elDesc.style.display = desc ? "" : "none";
    elDate.textContent = date;
    elDate.style.display = date ? "" : "none";
    elExif.innerHTML = exif;
    elExif.style.display = exif ? "" : "none";

    lb.classList.remove("hidden");
    lb.setAttribute("aria-hidden", "false");
    document.body.style.overflow = "hidden";
  }

  function close() {
    lb.classList.add("hidden");
    lb.setAttribute("aria-hidden", "true");
    media.innerHTML = "";
    document.body.style.overflow = "";
    current = -1;
  }

  function step(delta) {
    if (current < 0) return;
    let i = current + delta;
    if (i < 0) i = tiles.length - 1;
    if (i >= tiles.length) i = 0;
    show(i);
  }

  tiles.forEach((t, i) => {
    t.addEventListener("click", () => show(i));
  });
  btnClose.addEventListener("click", close);
  btnPrev.addEventListener("click", () => step(-1));
  btnNext.addEventListener("click", () => step(1));
  lb.addEventListener("click", (e) => {
    if (e.target === lb) close();
  });
  document.addEventListener("keydown", (e) => {
    if (lb.classList.contains("hidden")) return;
    if (e.key === "Escape") close();
    else if (e.key === "ArrowLeft") step(-1);
    else if (e.key === "ArrowRight") step(1);
  });
})();
