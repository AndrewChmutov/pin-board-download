# Default libraries
import json

# For browsing
import selenium
from selenium import webdriver
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.firefox.options import Options
from selenium.webdriver.common.by import By


url = input("Enter the url of Pinterest board: ")

# Firefox driver
driver_path = './geckodriver'
driver = webdriver.Firefox(
    service=Service(driver_path),
    options=Options()
)

# Get email and password from json
with open('info.json') as info_file:
    info = json.load(info_file)


# Open login window
driver.get(url)
login_button = driver.find_element(By.CSS_SELECTOR, "div[data-test-id='login-button']")
login_button.click()

# Find and fill the fields
email_field = driver.find_element(By.ID, 'email')
email_field.send_keys(info['login'])
password_field = driver.find_element(By.ID, 'password')
password_field.send_keys(info['pass'])

# Click login button again
login_button = driver.find_element(By.CLASS_NAME, 'SignupButton')
login_button.click()