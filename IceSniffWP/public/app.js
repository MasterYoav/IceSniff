const siteHeader = document.querySelector(".site-header");
const revealItems = document.querySelectorAll(".reveal");

if (siteHeader) {
  const syncHeaderState = () => {
    siteHeader.dataset.scrolled = window.scrollY > 12 ? "true" : "false";
  };

  syncHeaderState();
  window.addEventListener("scroll", syncHeaderState, { passive: true });
}

if ("IntersectionObserver" in window) {
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        entry.target.classList.add("is-visible");
        observer.unobserve(entry.target);
      });
    },
    {
      threshold: 0.2,
      rootMargin: "0px 0px -8% 0px",
    },
  );

  revealItems.forEach((item) => observer.observe(item));
} else {
  revealItems.forEach((item) => item.classList.add("is-visible"));
}
