FROM node:22
WORKDIR /app
COPY . .
RUN npm ci && npm run build
EXPOSE 5173
CMD ["npx", "vite", "preview", "--host", "0.0.0.0", "--port", "5173"]