FROM python:3.11-slim

WORKDIR /app

COPY pyproject.toml .
RUN pip install --no-cache-dir fastapi uvicorn pydantic

# Copy source code
COPY src/ ./src/

# Create data directory
RUN mkdir -p /app/data

# Set environment
ENV PYTHONPATH=/app/src

# Expose port
EXPOSE 8000

# Default command
CMD ["python", "-m", "film_record_lite.server", "--port", "8000"]