# Default libraries
import json

# For browsing
import selenium
from selenium import webdriver
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.firefox.options import Options


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