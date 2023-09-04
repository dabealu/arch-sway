package main

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"
)

const translateUrl = "https://translate.google.com/translate_a/single"

func main() {
	text := readInput()
	from, to := detectStringLang(text)

	translated, err := translate(from, to, text)
	if err != nil {
		log.Fatalf("failed to translate: %s", err)
	}

	fmt.Println(translated)
}

func readInput() string {
	// try to get input text from stdin...
	fi, err := os.Stdin.Stat()
	if err != nil {
		log.Fatalf("failed to get stdin file info: %s", err)
	}

	if (fi.Mode() & os.ModeCharDevice) == 0 {
		inBytes, err := io.ReadAll(os.Stdin)
		if err != nil {
			log.Fatalf("failed to read stdin: %s", err)
		}
		return string(inBytes)
	}

	// ...or fallback to args
	if len(os.Args) < 2 {
		log.Fatalf("usage: %s 'input text'", os.Args[0])
	}
	return strings.Join(os.Args[1:], " ")
}

// returns string's lang and lang to which it should be translated
func detectStringLang(s string) (string, string) {
	if len(s) == 0 {
		return "", ""
	}
	sym := s[0]
	// A-Z: 65-90, a-z: 97-122
	if (sym >= 65 && sym <= 90) || (sym >= 97 && sym <= 122) {
		return "en", "ru"
	}
	return "ru", "en"
}

type Sentence struct {
	Trans string `json:"trans"`
	Orig  string `json:"orig"`
}

type TranslationPayload struct {
	Sentences []Sentence `json:"sentences"`
}

func (tr *TranslationPayload) getSentences() string {
	trans := []string{}
	for _, s := range tr.Sentences {
		trans = append(trans, s.Trans)
	}
	return strings.Join(trans, " ")
}

func buildUrl(from, to, text string) string {
	query := fmt.Sprintf("client=at&dt=t&dj=1&sl=%s&tl=%s&q=%s", from, to, url.QueryEscape(text))
	return fmt.Sprintf("%s?%s", translateUrl, query)
}

func translate(from, to, text string) (string, error) {
	client := &http.Client{
		Timeout: time.Second * 5,
	}

	req, err := http.NewRequest("POST", buildUrl(from, to, text), nil)
	if err != nil {
		return "", err
	}
	req.Header.Add("Content-Type", "application/x-www-form-urlencoded;charset=utf-8")

	resp, err := client.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

	if resp.StatusCode != 200 {
		return "", fmt.Errorf("request failed %s: %s", resp.Status, string(body))
	}

	tr := &TranslationPayload{}
	if err := json.Unmarshal(body, tr); err != nil {
		return "", err
	}

	return tr.getSentences(), nil
}
