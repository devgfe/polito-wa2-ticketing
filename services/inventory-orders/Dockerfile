FROM eclipse-temurin:21-jdk
WORKDIR /app
COPY . .
RUN chmod +x gradlew && ./gradlew bootJar --no-daemon && cp build/libs/*.jar app.jar
CMD ["java", "-jar", "app.jar"]