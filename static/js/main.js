// Advent of Faith — main.js
// Gentle snowflake animation

document.addEventListener("DOMContentLoaded", () => {
    const snowflakeCount = 10;
    const body = document.body;
    const symbols = ["❄", "❅", "❆"];

    function createSnowflake() {
        const s = document.createElement("div");
        s.classList.add("snowflake");
        s.textContent = symbols[Math.floor(Math.random() * symbols.length)];

        const size = 10 + Math.random() * 16; // px
        s.style.fontSize = `${size}px`;
        s.style.left = Math.random() * 100 + "vw";
        s.style.animationDuration = 10 + Math.random() * 10 + "s";
        s.style.animationDelay = -Math.random() * 10 + "s";

        body.appendChild(s);
    }

    for (let i = 0; i < snowflakeCount; i++) createSnowflake();
    // setInterval(createSnowflake, 3000);
});
